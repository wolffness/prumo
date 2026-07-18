import Foundation

enum CaptureTagKind {
    case project
    case context

    var sigil: Character {
        switch self {
        case .project: "+"
        case .context: "@"
        }
    }
}

struct CaptureAutocompleteTarget {
    let kind: CaptureTagKind
    let prefix: String
    let replacementRange: Range<String.Index>
}

enum CaptureTagAutocomplete {
    static let matchLimit = 8

    static func target(in text: String, cursorUTF16: Int) -> CaptureAutocompleteTarget? {
        guard cursorUTF16 > 0 else { return nil }
        let clampedCursor = min(cursorUTF16, text.utf16.count)
        let cursor = String.Index(utf16Offset: clampedCursor, in: text)

        var start = cursor
        while start > text.startIndex {
            let previous = text.index(before: start)
            if text[previous].isWhitespace { break }
            start = previous
        }
        guard start < cursor else { return nil }

        let kind: CaptureTagKind
        switch text[start] {
        case "+": kind = .project
        case "@": kind = .context
        default: return nil
        }

        var end = cursor
        while end < text.endIndex, !text[end].isWhitespace {
            end = text.index(after: end)
        }

        let prefixStart = text.index(after: start)
        return CaptureAutocompleteTarget(
            kind: kind,
            prefix: String(text[prefixStart..<cursor]),
            replacementRange: start..<end
        )
    }

    static func values(in contents: String, kind: CaptureTagKind) -> [String] {
        var seen = Set<String>()
        var values: [String] = []

        for token in contents.split(whereSeparator: { $0.isWhitespace }) {
            guard token.first == kind.sigil, token.count > 1 else { continue }
            let value = String(token.dropFirst())
            if seen.insert(value).inserted {
                values.append(value)
            }
        }

        return values.sorted(by: caseInsensitiveOrder)
    }

    static func matches(prefix: String, in values: [String]) -> [String] {
        let prefixLower = prefix.lowercased()
        var prefixMatches: [String] = []
        var containsMatches: [String] = []

        for value in values.sorted(by: caseInsensitiveOrder) {
            let valueLower = value.lowercased()
            if valueLower.hasPrefix(prefixLower) {
                prefixMatches.append(value)
            } else if !prefixLower.isEmpty, valueLower.contains(prefixLower) {
                containsMatches.append(value)
            }
        }

        return Array((prefixMatches + containsMatches).prefix(matchLimit))
    }

    static func steppedSelection(current: Int, matchCount: Int, forward: Bool) -> Int {
        guard matchCount > 0 else { return 0 }
        let selected = min(max(0, current), matchCount - 1)
        return forward
            ? (selected + 1) % matchCount
            : (selected + matchCount - 1) % matchCount
    }

    static func replacing(
        target: CaptureAutocompleteTarget,
        with value: String,
        in text: String
    ) -> (text: String, cursorUTF16: Int) {
        var updated = text
        let replacement = String(target.kind.sigil) + value
        updated.replaceSubrange(target.replacementRange, with: replacement)
        let replacementEnd = target.replacementRange.lowerBound.utf16Offset(in: text)
            + replacement.utf16.count
        return (updated, replacementEnd)
    }

    @discardableResult
    static func appendCapture(_ text: String, to inbox: URL) -> Bool {
        let line = text + "\n"
        if let handle = try? FileHandle(forWritingTo: inbox) {
            handle.seekToEndOfFile()
            handle.write(Data(line.utf8))
            try? handle.close()
            return true
        }
        do {
            try Data(line.utf8).write(to: inbox)
            return true
        } catch {
            return false
        }
    }

    private static func caseInsensitiveOrder(_ lhs: String, _ rhs: String) -> Bool {
        let lhsLower = lhs.lowercased()
        let rhsLower = rhs.lowercased()
        return lhsLower == rhsLower ? lhs < rhs : lhsLower < rhsLower
    }
}
