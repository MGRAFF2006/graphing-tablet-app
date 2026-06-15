import Foundation

/// GraphPad binary wire protocol v1 (matches host/crates/graphpad-core).
enum GraphPadProtocol {
    static let magic: [UInt8] = Array("GPAD".utf8)
    static let version: UInt8 = 1
    static let defaultPort: UInt16 = 9470

    enum FrameType: UInt8 {
        case hello = 1
        case penDown = 2
        case penMove = 3
        case penUp = 4
        case penHover = 5
        case button = 6
        case heartbeat = 7
        case config = 8
        case hostHello = 9
    }

    struct PenEvent {
        var timestampUs: UInt64
        var xNorm: Float
        var yNorm: Float
        var pressure: Float
        var tiltX: Float
        var tiltY: Float
        var buttons: UInt8
        var sequence: UInt32
    }

    struct Hello {
        var protocolVersion: UInt16 = 1
        var deviceName: String
        var screenWidthMm: Float
        var screenHeightMm: Float
        var maxPressure: Float = 1.0
    }

    struct HostHello {
        var protocolVersion: UInt16
        var hostName: String
        var tabletWidth: UInt32
        var tabletHeight: UInt32
        var pressureMax: UInt32
    }

    enum PressureCurve: UInt8 {
        case linear = 0
        case soft = 1
        case firm = 2
    }

    enum MappingMode: UInt8 {
        case absolute = 0
        case relative = 1
    }

    struct Config {
        var mappingMode: MappingMode
        var pressureCurve: PressureCurve
        var targetMonitor: UInt32
        var areaLeft: Float
        var areaTop: Float
        var areaRight: Float
        var areaBottom: Float
    }

    static func encodeHello(_ hello: Hello) -> Data {
        var payload = Data()
        payload.appendUInt16BE(hello.protocolVersion)
        payload.appendString(hello.deviceName)
        payload.appendFloat32BE(hello.screenWidthMm)
        payload.appendFloat32BE(hello.screenHeightMm)
        payload.appendFloat32BE(hello.maxPressure)
        return wrap(frameType: .hello, payload: payload)
    }

    static func encodePenEvent(_ type: FrameType, _ event: PenEvent) -> Data {
        var payload = Data()
        payload.appendUInt64BE(event.timestampUs)
        payload.appendFloat32BE(event.xNorm)
        payload.appendFloat32BE(event.yNorm)
        payload.appendFloat32BE(event.pressure)
        payload.appendFloat32BE(event.tiltX)
        payload.appendFloat32BE(event.tiltY)
        payload.appendUInt8(event.buttons)
        payload.appendUInt32BE(event.sequence)
        return wrap(frameType: type, payload: payload)
    }

    static func encodeConfig(_ config: Config) -> Data {
        var payload = Data()
        payload.appendUInt8(config.mappingMode.rawValue)
        payload.appendUInt8(config.pressureCurve.rawValue)
        payload.appendUInt32BE(config.targetMonitor)
        payload.appendFloat32BE(config.areaLeft)
        payload.appendFloat32BE(config.areaTop)
        payload.appendFloat32BE(config.areaRight)
        payload.appendFloat32BE(config.areaBottom)
        return wrap(frameType: .config, payload: payload)
    }

    static func decodeHostHello(from data: Data) -> HostHello? {
        guard data.count >= 8 else { return nil }
        guard data.prefix(4).elementsEqual(magic) else { return nil }
        guard data[4] == version else { return nil }
        guard data[5] == FrameType.hostHello.rawValue else { return nil }

        let length = Int(data.readUInt16BE(at: 6))
        guard data.count >= 8 + length else { return nil }

        var offset = 8
        let protocolVersion = data.readUInt16BE(at: offset)
        offset += 2

        guard let (hostName, newOffset) = data.readString(at: offset) else { return nil }
        offset = newOffset

        guard offset + 12 <= 8 + length else { return nil }
        let tabletWidth = data.readUInt32BE(at: offset)
        offset += 4
        let tabletHeight = data.readUInt32BE(at: offset)
        offset += 4
        let pressureMax = data.readUInt32BE(at: offset)

        return HostHello(
            protocolVersion: protocolVersion,
            hostName: hostName,
            tabletWidth: tabletWidth,
            tabletHeight: tabletHeight,
            pressureMax: pressureMax
        )
    }

    private static func wrap(frameType: FrameType, payload: Data) -> Data {
        var frame = Data()
        frame.append(contentsOf: magic)
        frame.append(version)
        frame.append(frameType.rawValue)
        frame.appendUInt16BE(UInt16(payload.count))
        frame.append(payload)
        return frame
    }
}

private extension Data {
    mutating func appendUInt8(_ value: UInt8) {
        append(value)
    }

    mutating func appendUInt16BE(_ value: UInt16) {
        var v = value.bigEndian
        append(Data(bytes: &v, count: 2))
    }

    mutating func appendUInt32BE(_ value: UInt32) {
        var v = value.bigEndian
        append(Data(bytes: &v, count: 4))
    }

    mutating func appendUInt64BE(_ value: UInt64) {
        var v = value.bigEndian
        append(Data(bytes: &v, count: 8))
    }

    mutating func appendFloat32BE(_ value: Float) {
        appendUInt32BE(value.bitPattern)
    }

    mutating func appendString(_ value: String) {
        let bytes = Array(value.utf8)
        appendUInt16BE(UInt16(bytes.count))
        append(contentsOf: bytes)
    }

    func readUInt16BE(at offset: Int) -> UInt16 {
        let slice = self[offset..<offset + 2]
        return UInt16(slice[slice.startIndex]) << 8 | UInt16(slice[slice.startIndex + 1])
    }

    func readUInt32BE(at offset: Int) -> UInt32 {
        var value: UInt32 = 0
        for i in 0..<4 {
            value = (value << 8) | UInt32(self[offset + i])
        }
        return value
    }

    func readUInt64BE(at offset: Int) -> UInt64 {
        var value: UInt64 = 0
        for i in 0..<8 {
            value = (value << 8) | UInt64(self[offset + i])
        }
        return value
    }

    func readString(at offset: Int) -> (String, Int)? {
        guard offset + 2 <= count else { return nil }
        let length = Int(readUInt16BE(at: offset))
        let start = offset + 2
        guard start + length <= count else { return nil }
        let bytes = self[start..<start + length]
        guard let string = String(bytes: bytes, encoding: .utf8) else { return nil }
        return (string, start + length)
    }
}

private extension Float {
    var bitPattern: UInt32 {
        withUnsafeBytes(of: self) { $0.load(as: UInt32.self) }
    }
}
