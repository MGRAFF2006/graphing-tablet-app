import Foundation
import UIKit

enum PencilPhase: UInt8 {
    case down = 2
    case move = 3
    case up = 4
    case hover = 5
}

struct PencilSample {
    let phase: PencilPhase
    let timestampUs: UInt64
    let xNorm: Float
    let yNorm: Float
    let pressure: Float
    let tiltX: Float
    let tiltY: Float
    let buttons: UInt8
    let sequence: UInt32
}

enum GraphPadProtocol {
    static let version: UInt8 = 1
    static let helloKind: UInt8 = 1
    static let heartbeatKind: UInt8 = 7

    static func hello(deviceName: String, screenSize: CGSize) -> Data {
        var payload = Data()
        payload.appendUInt16(UInt16(version))
        payload.appendUInt32(UInt32(max(screenSize.width.rounded(), 1)))
        payload.appendUInt32(UInt32(max(screenSize.height.rounded(), 1)))
        payload.appendUInt16(4096)
        payload.appendString(deviceName)
        return frame(kind: helloKind, payload: payload)
    }

    static func pen(_ sample: PencilSample) -> Data {
        var payload = Data()
        payload.appendUInt64(sample.timestampUs)
        payload.appendFloat32(sample.xNorm)
        payload.appendFloat32(sample.yNorm)
        payload.appendFloat32(sample.pressure)
        payload.appendFloat32(sample.tiltX)
        payload.appendFloat32(sample.tiltY)
        payload.append(sample.buttons)
        payload.appendUInt32(sample.sequence)
        return frame(kind: sample.phase.rawValue, payload: payload)
    }

    static func heartbeat() -> Data {
        var payload = Data()
        payload.appendUInt64(UInt64(Date().timeIntervalSince1970 * 1_000_000))
        return frame(kind: heartbeatKind, payload: payload)
    }

    private static func frame(kind: UInt8, payload: Data) -> Data {
        precondition(payload.count <= UInt16.max)

        var data = Data()
        data.append(contentsOf: [0x47, 0x50, 0x41, 0x44])
        data.append(version)
        data.append(kind)
        data.appendUInt16(UInt16(payload.count))
        data.append(payload)
        return data
    }
}

private extension Data {
    mutating func appendUInt16(_ value: UInt16) {
        var bigEndian = value.bigEndian
        appendBytes(of: &bigEndian)
    }

    mutating func appendUInt32(_ value: UInt32) {
        var bigEndian = value.bigEndian
        appendBytes(of: &bigEndian)
    }

    mutating func appendUInt64(_ value: UInt64) {
        var bigEndian = value.bigEndian
        appendBytes(of: &bigEndian)
    }

    mutating func appendFloat32(_ value: Float) {
        appendUInt32(value.bitPattern)
    }

    mutating func appendString(_ value: String) {
        let bytes = Array(value.utf8.prefix(Int(UInt16.max)))
        appendUInt16(UInt16(bytes.count))
        append(contentsOf: bytes)
    }

    mutating func appendBytes<T>(of value: inout T) {
        Swift.withUnsafeBytes(of: &value) { append(contentsOf: $0) }
    }
}

