// A dropdown row with TWO independent click zones: a circle on the left that
// completes the task (turning into a check ✓ on hover, as a "click to complete"
// affordance) and the text, which opens the task. A standard NSMenuItem is a
// single click target, so the row is a custom NSView instead.
import AppKit

final class TaskRowView: NSView {
    private let text: String
    private let trailing: String            // "−4d" for overdue, "" otherwise
    private let onComplete: () -> Void
    private let onOpen: () -> Void

    private var hovering = false
    private var overCircle = false
    private let circleZoneWidth: CGFloat = 30
    private let font = NSFont(name: "IBMPlexMono-Regular", size: 13)
        ?? NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)

    init(text: String, trailing: String,
         onComplete: @escaping () -> Void, onOpen: @escaping () -> Void) {
        self.text = text
        self.trailing = trailing
        self.onComplete = onComplete
        self.onOpen = onOpen
        super.init(frame: NSRect(x: 0, y: 0, width: 360, height: 24))
    }
    required init?(coder: NSCoder) { fatalError("not used") }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(
            rect: bounds,
            options: [.mouseEnteredAndExited, .mouseMoved, .activeAlways],
            owner: self, userInfo: nil))
    }

    override func mouseEntered(with e: NSEvent) { hovering = true; needsDisplay = true }
    override func mouseExited(with e: NSEvent) {
        hovering = false; overCircle = false; needsDisplay = true
    }
    override func mouseMoved(with e: NSEvent) {
        let oc = convert(e.locationInWindow, from: nil).x <= circleZoneWidth
        if oc != overCircle { overCircle = oc; needsDisplay = true }
    }

    override func mouseUp(with e: NSEvent) {
        let inCircle = convert(e.locationInWindow, from: nil).x <= circleZoneWidth
        enclosingMenuItem?.menu?.cancelTracking()   // close the menu first
        if inCircle { onComplete() } else { onOpen() }
    }

    override func draw(_ dirtyRect: NSRect) {
        if hovering {
            NSColor.selectedContentBackgroundColor.setFill()
            NSBezierPath(roundedRect: bounds.insetBy(dx: 4, dy: 1),
                         xRadius: 5, yRadius: 5).fill()
        }
        let onHi = hovering
        // Circle → check on circle-hover.
        let glyph = overCircle ? "✓" : "○"
        let circleColor: NSColor = overCircle ? Theme.phosphor
            : (onHi ? .white : Theme.phosphorDim)
        draw(glyph, x: 10, maxX: circleZoneWidth, color: circleColor, bold: overCircle)
        // Task text.
        draw(text, x: 32, maxX: bounds.width - 44,
             color: onHi ? .white : .labelColor, bold: false)
        // Overdue badge, right-aligned.
        if !trailing.isEmpty {
            let attrs: [NSAttributedString.Key: Any] = [
                .font: font, .foregroundColor: onHi ? NSColor.white : Theme.amber]
            let s = NSAttributedString(string: trailing, attributes: attrs)
            s.draw(at: NSPoint(x: bounds.width - s.size().width - 12,
                               y: (bounds.height - s.size().height) / 2))
        }
    }

    private func draw(_ str: String, x: CGFloat, maxX: CGFloat,
                      color: NSColor, bold: Bool) {
        let f = bold ? (NSFont(name: "IBMPlexMono-Bold", size: 13) ?? font) : font
        let para = NSMutableParagraphStyle()
        para.lineBreakMode = .byTruncatingTail
        let attrs: [NSAttributedString.Key: Any] = [
            .font: f, .foregroundColor: color, .paragraphStyle: para]
        let s = NSAttributedString(string: str, attributes: attrs)
        let h = s.size().height
        s.draw(in: NSRect(x: x, y: (bounds.height - h) / 2, width: maxX - x, height: h))
    }
}
