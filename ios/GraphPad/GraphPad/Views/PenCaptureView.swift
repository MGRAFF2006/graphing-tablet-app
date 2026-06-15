import SwiftUI
import UIKit

struct PenCaptureView: UIViewRepresentable {
    @ObservedObject var connection: ConnectionManager
    var pressureCurve: GraphPadProtocol.PressureCurve
    var pencilOnly: Bool

    func makeUIView(context: Context) -> PenCaptureUIView {
        let view = PenCaptureUIView()
        view.delegate = context.coordinator
        return view
    }

    func updateUIView(_ uiView: PenCaptureUIView, context: Context) {
        context.coordinator.connection = connection
        context.coordinator.pressureCurve = pressureCurve
        context.coordinator.pencilOnly = pencilOnly
        uiView.pencilOnly = pencilOnly
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(connection: connection, pressureCurve: pressureCurve, pencilOnly: pencilOnly)
    }

    final class Coordinator: NSObject, PenCaptureDelegate {
        var connection: ConnectionManager
        var pressureCurve: GraphPadProtocol.PressureCurve
        var pencilOnly: Bool

        init(
            connection: ConnectionManager,
            pressureCurve: GraphPadProtocol.PressureCurve,
            pencilOnly: Bool
        ) {
            self.connection = connection
            self.pressureCurve = pressureCurve
            self.pencilOnly = pencilOnly
        }

        func penCapture(
            _ view: PenCaptureUIView,
            didUpdate touches: [PenTouchSample],
            phase: PenPhase,
            in bounds: CGRect
        ) {
            guard connection.isConnected else { return }

            for sample in touches {
                let event = makeEvent(from: sample, bounds: bounds)
                switch phase {
                case .began:
                    connection.sendPenDown(event)
                case .moved:
                    connection.sendPenMove(event)
                case .ended, .cancelled:
                    connection.sendPenUp(event)
                case .hover:
                    connection.sendPenHover(event)
                }
            }
        }

        private func makeEvent(from sample: PenTouchSample, bounds: CGRect) -> GraphPadProtocol.PenEvent {
            let xNorm = Float(sample.location.x / max(bounds.width, 1))
            let yNorm = Float(sample.location.y / max(bounds.height, 1))
            let pressure = applyPressureCurve(sample.pressure)

            return GraphPadProtocol.PenEvent(
                timestampUs: sample.timestampUs,
                xNorm: min(max(xNorm, 0), 1),
                yNorm: min(max(yNorm, 0), 1),
                pressure: pressure,
                tiltX: sample.tiltX,
                tiltY: sample.tiltY,
                buttons: sample.buttons,
                sequence: 0
            )
        }

        private func applyPressureCurve(_ pressure: Float) -> Float {
            let p = min(max(pressure, 0), 1)
            switch pressureCurve {
            case .linear:
                return p
            case .soft:
                return p * p
            case .firm:
                return sqrtf(p)
            }
        }
    }
}

enum PenPhase {
    case began, moved, ended, cancelled, hover
}

struct PenTouchSample {
    var location: CGPoint
    var pressure: Float
    var tiltX: Float
    var tiltY: Float
    var buttons: UInt8
    var timestampUs: UInt64
}

protocol PenCaptureDelegate: AnyObject {
    func penCapture(
        _ view: PenCaptureUIView,
        didUpdate touches: [PenTouchSample],
        phase: PenPhase,
        in bounds: CGRect
    )
}

final class PenCaptureUIView: UIView {
    weak var delegate: PenCaptureDelegate?
    var pencilOnly = true

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = false
        backgroundColor = UIColor(white: 0.08, alpha: 1)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        send(touches: touches, event: event, phase: .began)
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        send(touches: touches, event: event, phase: .moved)
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        send(touches: touches, event: event, phase: .ended)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        send(touches: touches, event: event, phase: .cancelled)
    }

    private func send(touches: Set<UITouch>, event: UIEvent?, phase: PenPhase) {
        let coalesced = event?.coalescedTouches(for: touches.first!) ?? Array(touches)
        let samples = coalesced.compactMap { sample(from: $0) }
        guard !samples.isEmpty else { return }
        delegate?.penCapture(self, didUpdate: samples, phase: phase, in: bounds)
    }

    private func sample(from touch: UITouch) -> PenTouchSample? {
        if pencilOnly && touch.type != .pencil && touch.type != .stylus {
            return nil
        }
        guard touch.type == .pencil || touch.type == .stylus || !pencilOnly else { return nil }

        let location = touch.location(in: self)
        let force = touch.force
        let maxForce = touch.maximumPossibleForce
        let pressure = maxForce > 0 ? Float(force / maxForce) : (touch.type == .pencil ? 1 : 0)

        var tiltX: Float = 0
        var tiltY: Float = 0
        if touch.type == .pencil {
            let altitude = touch.altitudeAngle
            let azimuth = touch.azimuthAngle(in: self)
            tiltX = Float(cos(azimuth) * sin(altitude))
            tiltY = Float(sin(azimuth) * sin(altitude))
        }

        let timestampUs = UInt64(touch.timestamp * 1_000_000)

        return PenTouchSample(
            location: location,
            pressure: pressure,
            tiltX: tiltX,
            tiltY: tiltY,
            buttons: 0,
            timestampUs: timestampUs
        )
    }
}
