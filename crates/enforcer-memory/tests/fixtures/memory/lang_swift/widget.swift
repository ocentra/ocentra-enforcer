import Foundation

protocol Drawable {
    func draw() -> String
}

class Widget: Drawable {
    let name: String

    init(name: String) {
        self.name = name
    }

    func draw() -> String {
        if name.isEmpty {
            return "unnamed"
        }
        return helper(name)
    }
}

func helper(_ label: String) -> String {
    return label.uppercased()
}
