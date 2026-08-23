import Foundation

@main
private struct ThemeCLI {
    static func main() {
        let arguments = Array(CommandLine.arguments.dropFirst())
        guard let command = arguments.first else { usage() }

        do {
            switch command {
            case "validate":
                guard arguments.count == 2 else { usage() }
                let url = URL(fileURLWithPath: arguments[1]).standardizedFileURL
                let document = try PhotaraThemeDocument.load(from: url)
                print("valid: \(document.id) (\(document.displayName))")
                for warning in document.contrastWarnings() {
                    print("warning: \(warning)")
                }
            case "use":
                guard arguments.count == 2 else { usage() }
                let url = URL(fileURLWithPath: arguments[1]).standardizedFileURL
                let document = try PhotaraThemeDocument.load(from: url)
                PhotaraThemeDevelopmentSettings.setOverrideURL(url)
                print("using: \(document.id) at \(url.path)")
            case "reset":
                guard arguments.count == 1 else { usage() }
                PhotaraThemeDevelopmentSettings.setOverrideURL(nil)
                print("using bundled Photara theme")
            case "current":
                guard arguments.count == 1 else { usage() }
                print(PhotaraThemeDevelopmentSettings.overrideURL?.path ?? "bundled")
            default:
                usage()
            }
        } catch {
            FileHandle.standardError.write(Data("error: \(error.localizedDescription)\n".utf8))
            exit(1)
        }
    }

    private static func usage() -> Never {
        FileHandle.standardError.write(Data("""
        usage:
          photara-theme validate THEME.json
          photara-theme use THEME.json
          photara-theme reset
          photara-theme current

        """.utf8))
        exit(2)
    }
}
