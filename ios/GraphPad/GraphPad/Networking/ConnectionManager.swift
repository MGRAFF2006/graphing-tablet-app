import Foundation
import Network

@MainActor
final class ConnectionManager: ObservableObject {
    @Published var isConnected = false
    @Published var statusMessage = "Not connected"
    @Published var hostName = ""
    @Published var lastSequence: UInt32 = 0

    private var connection: NWConnection?
    private var receiveBuffer = Data()
    private var sequence: UInt32 = 0

    var hostAddress: String = ""
    var port: UInt16 = GraphPadProtocol.defaultPort

    func connect(to host: String, port: UInt16 = GraphPadProtocol.defaultPort) {
        disconnect()
        hostAddress = host
        self.port = port

        let endpoint = NWEndpoint.hostPort(
            host: NWEndpoint.Host(host),
            port: NWEndpoint.Port(rawValue: port)!
        )

        let parameters = NWParameters.tcp
        parameters.includePeerToPeer = true

        let conn = NWConnection(to: endpoint, using: parameters)
        connection = conn
        statusMessage = "Connecting to \(host):\(port)..."

        conn.stateUpdateHandler = { [weak self] state in
            Task { @MainActor in
                self?.handleState(state)
            }
        }

        conn.start(queue: .global(qos: .userInteractive))
        startReceiving()
    }

    func disconnect() {
        connection?.cancel()
        connection = nil
        isConnected = false
        statusMessage = "Disconnected"
        hostName = ""
    }

    func sendHello(deviceName: String, widthMm: Float, heightMm: Float) {
        let hello = GraphPadProtocol.Hello(
            deviceName: deviceName,
            screenWidthMm: widthMm,
            screenHeightMm: heightMm
        )
        send(GraphPadProtocol.encodeHello(hello))
    }

    func sendConfig(_ config: GraphPadProtocol.Config) {
        send(GraphPadProtocol.encodeConfig(config))
    }

    func sendPenDown(_ event: GraphPadProtocol.PenEvent) {
        sequence &+= 1
        var e = event
        e.sequence = sequence
        lastSequence = sequence
        send(GraphPadProtocol.encodePenEvent(.penDown, e))
    }

    func sendPenMove(_ event: GraphPadProtocol.PenEvent) {
        sequence &+= 1
        var e = event
        e.sequence = sequence
        lastSequence = sequence
        send(GraphPadProtocol.encodePenEvent(.penMove, e))
    }

    func sendPenUp(_ event: GraphPadProtocol.PenEvent) {
        sequence &+= 1
        var e = event
        e.sequence = sequence
        lastSequence = sequence
        send(GraphPadProtocol.encodePenEvent(.penUp, e))
    }

    func sendPenHover(_ event: GraphPadProtocol.PenEvent) {
        sequence &+= 1
        var e = event
        e.sequence = sequence
        lastSequence = sequence
        send(GraphPadProtocol.encodePenEvent(.penHover, e))
    }

    private func send(_ data: Data) {
        guard let connection else { return }
        connection.send(content: data, completion: .contentProcessed { error in
            if let error {
                Task { @MainActor in
                    self.statusMessage = "Send error: \(error.localizedDescription)"
                }
            }
        })
    }

    private func handleState(_ state: NWConnection.State) {
        switch state {
        case .ready:
            isConnected = true
            statusMessage = "Connected"
        case .waiting(let error):
            statusMessage = "Waiting: \(error.localizedDescription)"
        case .failed(let error):
            isConnected = false
            statusMessage = "Failed: \(error.localizedDescription)"
        case .cancelled:
            isConnected = false
            statusMessage = "Disconnected"
        default:
            break
        }
    }

    private func startReceiving() {
        guard let connection else { return }

        connection.receive(minimumIncompleteLength: 1, maximumLength: 65536) { [weak self] data, _, isComplete, error in
            Task { @MainActor in
                guard let self else { return }

                if let data, !data.isEmpty {
                    self.receiveBuffer.append(data)
                    self.processBuffer()
                }

                if let error {
                    self.statusMessage = "Receive error: \(error.localizedDescription)"
                    self.isConnected = false
                    return
                }

                if isComplete {
                    self.isConnected = false
                    self.statusMessage = "Connection closed"
                    return
                }

                self.startReceiving()
            }
        }
    }

    private func processBuffer() {
        while receiveBuffer.count >= 8 {
            guard receiveBuffer.prefix(4).elementsEqual(GraphPadProtocol.magic) else {
                receiveBuffer.removeFirst()
                continue
            }

            let length = Int(receiveBuffer.readUInt16BE(at: 6))
            let frameLen = 8 + length
            guard receiveBuffer.count >= frameLen else { break }

            let frame = receiveBuffer.prefix(frameLen)
            if frame[5] == GraphPadProtocol.FrameType.hostHello.rawValue,
               let hello = GraphPadProtocol.decodeHostHello(from: frame) {
                hostName = hello.hostName
                statusMessage = "Connected to \(hello.hostName)"
            }

            receiveBuffer.removeFirst(frameLen)
        }
    }
}

private extension Data {
    func readUInt16BE(at offset: Int) -> UInt16 {
        let slice = self[offset..<offset + 2]
        return UInt16(slice[slice.startIndex]) << 8 | UInt16(slice[slice.startIndex + 1])
    }
}
