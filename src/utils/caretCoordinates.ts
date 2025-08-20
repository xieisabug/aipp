export function getCaretCoordinates(element: HTMLTextAreaElement, position: number) {
    const div = document.createElement("div");
    const style = div.style;
    const computed = window.getComputedStyle(element);

    style.whiteSpace = "pre-wrap";
    style.wordWrap = "break-word";
    style.position = "absolute";
    style.visibility = "hidden";

    for (const prop of [
        "direction",
        "boxSizing",
        "width",
        "height",
        "overflowX",
        "overflowY",
        "borderTopWidth",
        "borderRightWidth",
        "borderBottomWidth",
        "borderLeftWidth",
        "paddingTop",
        "paddingRight",
        "paddingBottom",
        "paddingLeft",
        "fontStyle",
        "fontVariant",
        "fontWeight",
        "fontStretch",
        "fontSize",
        "fontSizeAdjust",
        "lineHeight",
        "fontFamily",
        "textAlign",
        "textTransform",
        "textIndent",
        "textDecoration",
        "letterSpacing",
        "wordSpacing",
    ]) {
        style[prop as any] = computed[prop as any];
    }

    const text = element.value.substring(0, position);
    const span = document.createElement("span");
    span.textContent = text;
    div.appendChild(span);

    const cursorSpan = document.createElement("span");
    cursorSpan.textContent = "|";
    div.appendChild(cursorSpan);

    document.body.appendChild(div);
    const coordinates = {
        left: span.offsetLeft,
        top: span.offsetTop,
        height: span.offsetHeight,
        width: span.offsetWidth,
        cursorLeft: cursorSpan.offsetLeft,
        cursorTop: cursorSpan.offsetTop,
    };
    document.body.removeChild(div);

    return coordinates;
}
