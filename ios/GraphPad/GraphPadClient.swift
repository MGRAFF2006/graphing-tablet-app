import Foundation
import Network
import UIKit

final class GraphPadClient {
    var onStateChange: ((String) -> Void)?

    private let host: NWEndpoint.Host
    private let port: NWEndpoint.Port
    private var connection: NWConnection?
    private let queue = DispatchQueue(label: "app.graphpad.client")
    private var lastScreenSize: CGSize = .zero

    init(host: String, port: UInt16) {
        self.host = NWEndpoint.Host(host)
        self.port = NWEndpoint.Port(rawValue: port) ?? 47391
    }

    func connect() {
        let parameters = NWParameters.tcp
        parameters.allowLocalEndpointReuse = true

        let connection = NWConnection(host: host, port: port, using: parameters)
        self.connection = connection
        connection.stateUpdateHandler = { [weak self] state in
            switch state {
            case .ready:
                self?.onStateChange?("Connected")
                self?.sendHelloIfReady()
            case .failed(let error):
                self?.onStateChange?("Failed: \(error.localizedDescription)")
            case .waiting(let error):
                self?.onStateChange?("Waiting: \(error.localizedDescription)")
            case .cancelled:
                self?.onStateChange?("Disconnected")
            default:
                break
            }
        }
        connection.start(queue: queue)
    }

    func disconnect() {
        connection?.cancel()
        connection = nil
    }

    func sendHello(screenSize: CGSize) {
        lastScreenSize = screenSize
        sendHelloIfReady()
    }

    func send(_ sample: PencilSample) {
        send(GraphPadProtocol.pen(sample))
    }

    private func sendHelloIfReady() {
        guard lastScreenSize != .zero else { return }
        send(GraphPadProtocol.hello(deviceName: UIDevice.current.name, screenSize: lastScreenSize))
    }

    private func send(_ data: Data) {
        queue.async { [weak self] in
            guard let connection = self?.connection else { return }
            connection.send(content: data, completion: .contentProcessed { error in
                if let error {
                    self?.onStateChange?("Send failed: \(error.localizedDescription)")
                }
            })
        }
    }
}

