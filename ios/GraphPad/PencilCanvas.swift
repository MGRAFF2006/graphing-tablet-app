import SwiftUI
import UIKit

struct PencilCanvas: UIViewRepresentable {
    @ObservedObject var model: GraphPadModel

    func makeUIView(context: Context) -> PencilCanvasView {
        let view = PencilCanvasView()
        view.backgroundColor = .clear
        view.isMultipleTouchEnabled = true
        view.onSamples = { samples in
            Task { @MainActor in
                for sample in samples {
                    model.send(sample)
                }
            }
        }
        view.onReady = { size in
            Task { @MainActor in
                model.sendHello(screenSize: size)
            }
        }
        view.pencilOnly = model.pencilOnly
        return view
    }

    func updateUIView(_ uiView: PencilCanvasView, context: Context) {
        uiView.pencilOnly = model.pencilOnly
    }
}

final class PencilCanvasView: UIView {
    var onSamples: (([PencilSample]) -> Void)?
    var onReady: ((CGSize) -> Void)?
    var pencilOnly: Bool = true

    private var sequence: UInt32 = 0
    private var didReportSize = false

    override func layoutSubviews() {
        super.layoutSubviews()
        if !didReportSize, bounds.width > 0, bounds.height > 0 {
            didReportSize = true
            onReady?(bounds.size)
        }
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        emitSamples(from: touches, with: event, phase: .down)
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        emitSamples(from: touches, with: event, phase: .move)
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        emitSamples(from: touches, with: event, phase: .up)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        emitSamples(from: touches, with: event, phase: .up)
    }

    private func emitSamples(from touches: Set<UITouch>, with event: UIEvent?, phase: PencilPhase) {
        var samples: [PencilSample] = []

        for touch in touches {
            guard shouldUse(touch) else { continue }
            let coalesced = event?.coalescedTouches(for: touch) ?? [touch]
            for coalescedTouch in coalesced where shouldUse(coalescedTouch) {
                samples.append(sample(from: coalescedTouch, phase: phase))
            }
        }

        if !samples.isEmpty {
            onSamples?(samples)
        }
    }

    private func shouldUse(_ touch: UITouch) -> Bool {
        !pencilOnly || touch.type == .pencil
    }

    private func sample(from touch: UITouch, phase: PencilPhase) -> PencilSample {
        sequence &+= 1
        let point = touch.location(in: self)
        let pressure = touch.maximumPossibleForce > 0
            ? Float(touch.force / touch.maximumPossibleForce)
            : 0
        let tilt = tiltComponents(from: touch)

        return PencilSample(
            phase: phase,
            timestampUs: UInt64(touch.timestamp * 1_000_000),
            xNorm: Float((point.x / max(bounds.width, 1)).clamped(to: 0...1)),
            yNorm: Float((point.y / max(bounds.height, 1)).clamped(to: 0...1)),
            pressure: pressure.clamped(to: 0...1),
            tiltX: tilt.x,
            tiltY: tilt.y,
            buttons: 0,
            sequence: sequence
        )
    }

    private func tiltComponents(from touch: UITouch) -> (x: Float, y: Float) {
        let altitude = touch.altitudeAngle
        let azimuth = touch.azimuthAngle(in: self)
        let normalizedMagnitude = Float((1 - (altitude / (.pi / 2))).clamped(to: 0...1))
        return (
            x: Float(cos(azimuth)) * normalizedMagnitude,
            y: Float(sin(azimuth)) * normalizedMagnitude
        )
    }
}

private extension Comparable {
    func clamped(to range: ClosedRange<Self>) -> Self {
        min(max(self, range.lowerBound), range.upperBound)
    }
}

