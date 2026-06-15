import SwiftUI

@main
struct GraphPadApp: App {
    @StateObject private var model = GraphPadModel()

    var body: some Scene {
        WindowGroup {
            ContentView(model: model)
        }
    }
}

