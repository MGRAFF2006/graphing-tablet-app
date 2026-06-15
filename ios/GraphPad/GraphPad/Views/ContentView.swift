import SwiftUI
import UIKit

struct ContentView: View {
    @StateObject private var connection = ConnectionManager()
    @State private var hostAddress = ""
    @State private var showSettings = false
    @State private var pressureCurve: GraphPadProtocol.PressureCurve = .linear
    @State private var pencilOnly = true
    @State private var selectedTab = 0

    var body: some View {
        TabView(selection: $selectedTab) {
            drawingTab
                .tabItem {
                    Label("Draw", systemImage: "pencil.tip")
                }
                .tag(0)

            connectionTab
                .tabItem {
                    Label("Connect", systemImage: "cable.connector")
                }
                .tag(1)

            settingsTab
                .tabItem {
                    Label("Settings", systemImage: "slider.horizontal.3")
                }
                .tag(2)
        }
        .onChange(of: connection.isConnected) { connected in
            if connected {
                connection.sendHello(
                    deviceName: UIDevice.current.name,
                    widthMm: 214,
                    heightMm: 273
                )
                connection.sendConfig(
                    GraphPadProtocol.Config(
                        mappingMode: .absolute,
                        pressureCurve: pressureCurve,
                        targetMonitor: 0,
                        areaLeft: 0,
                        areaTop: 0,
                        areaRight: 1,
                        areaBottom: 1
                    )
                )
            }
        }
    }

    private var drawingTab: some View {
        VStack(spacing: 0) {
            HStack {
                Circle()
                    .fill(connection.isConnected ? Color.green : Color.red)
                    .frame(width: 10, height: 10)
                Text(connection.isConnected ? "Connected to \(connection.hostName)" : "Not connected")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer()
                if connection.isConnected {
                    Text("seq \(connection.lastSequence)")
                        .font(.caption.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            PenCaptureView(
                connection: connection,
                pressureCurve: pressureCurve,
                pencilOnly: pencilOnly
            )
        }
    }

    private var connectionTab: some View {
        Form {
            Section("Host") {
                TextField("Host IP or hostname", text: $hostAddress)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.decimalPad)

                HStack {
                    Button(connection.isConnected ? "Disconnect" : "Connect") {
                        if connection.isConnected {
                            connection.disconnect()
                        } else {
                            connection.connect(to: hostAddress)
                        }
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(hostAddress.isEmpty && !connection.isConnected)

                    Spacer()

                    Text(connection.statusMessage)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            Section("USB Tethering") {
                Text("Connect iPad to your computer with a USB-C cable, enable Personal Hotspot (or allow network access when prompted), then enter the host IP shown by graphpad-host.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Section("Wi-Fi") {
                Text("Ensure iPad and host are on the same subnet. The host listens on port \(GraphPadProtocol.defaultPort).")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var settingsTab: some View {
        Form {
            Section("Input") {
                Toggle("Pencil only (palm rejection)", isOn: $pencilOnly)

                Picker("Pressure curve", selection: $pressureCurve) {
                    Text("Linear").tag(GraphPadProtocol.PressureCurve.linear)
                    Text("Soft").tag(GraphPadProtocol.PressureCurve.soft)
                    Text("Firm").tag(GraphPadProtocol.PressureCurve.firm)
                }
            }

            Section("About") {
                LabeledContent("Protocol", value: "GraphPad v1")
                LabeledContent("Port", value: "\(GraphPadProtocol.defaultPort)")
            }
        }
    }
}

#Preview {
    ContentView()
}
