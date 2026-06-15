import SwiftUI

struct ContentView: View {
    @ObservedObject var model: GraphPadModel

    var body: some View {
        VStack(spacing: 16) {
            connectionPanel
            PencilCanvas(model: model)
                .background(Color(white: 0.08))
                .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
                .overlay(alignment: .topLeading) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("Apple Pencil capture surface")
                            .font(.headline)
                        Text(model.pencilOnly ? "Pencil-only mode" : "Touch and Pencil mode")
                            .font(.caption)
                    }
                    .foregroundStyle(.white)
                    .padding(16)
                }
        }
        .padding(20)
        .background(Color(white: 0.14))
    }

    private var connectionPanel: some View {
        HStack(spacing: 12) {
            TextField("Host IP", text: $model.host)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 260)

            TextField("Port", text: $model.port)
                .keyboardType(.numberPad)
                .textFieldStyle(.roundedBorder)
                .frame(width: 90)

            Toggle("Pencil only", isOn: $model.pencilOnly)
                .toggleStyle(.switch)
                .frame(width: 170)

            Button(model.isConnected ? "Disconnect" : "Connect") {
                model.isConnected ? model.disconnect() : model.connect()
            }
            .buttonStyle(.borderedProminent)

            Text(model.status)
                .foregroundStyle(model.isConnected ? .green : .secondary)

            Spacer()
        }
        .padding(16)
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 16, style: .continuous))
    }
}

#Preview {
    ContentView(model: GraphPadModel())
}

