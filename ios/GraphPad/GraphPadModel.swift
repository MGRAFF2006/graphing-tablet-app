import Combine
import UIKit

@MainActor
final class GraphPadModel: ObservableObject {
    @Published var host: String = ""
    @Published var port: String = "47391"
    @Published var status: String = "Disconnected"
    @Published var isConnected: Bool = false
    @Published var pencilOnly: Bool = true

    private var client: GraphPadClient?

    func connect() {
        guard let portNumber = UInt16(port) else {
            status = "Invalid port"
            return
        }

        let client = GraphPadClient(host: host, port: portNumber)
        self.client = client
        status = "Connecting..."
        client.onStateChange = { [weak self] state in
            Task { @MainActor in
                self?.status = state
                self?.isConnected = state == "Connected"
            }
        }
        client.connect()
    }

    func disconnect() {
        client?.disconnect()
        client = nil
        status = "Disconnected"
        isConnected = false
    }

    func send(_ sample: PencilSample) {
        client?.send(sample)
    }

    func sendHello(screenSize: CGSize) {
        client?.sendHello(screenSize: screenSize)
    }
}

