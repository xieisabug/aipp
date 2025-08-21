import React from "react";

interface BangCompletionListProps {
    bangListVisible: boolean;
    placement: "top" | "bottom";
    cursorPosition: {
        bottom: number;
        left: number;
        top: number;
    };
    bangList: string[];
    selectedBangIndex: number;
    textareaRef: React.RefObject<HTMLTextAreaElement | null>;
    setInputText: (value: string) => void;
    setBangListVisible: (value: boolean) => void;
}

const BangCompletionList: React.FC<BangCompletionListProps> = ({
    bangListVisible,
    placement,
    cursorPosition,
    bangList,
    selectedBangIndex,
    textareaRef,
    setInputText,
    setBangListVisible,
}) => {
    if (!bangListVisible) return null;

    return (
        <div
            className="completion-bang-list"
            style={{
                ...(placement === "top"
                    ? { top: cursorPosition.top }
                    : { bottom: cursorPosition.bottom }),
                left: cursorPosition.left,
            }}
        >
            {bangList.map(([bang, complete, desc], index) => (
                <div
                    className={`completion-bang-container ${
                        index === selectedBangIndex ? "selected" : ""
                    }`}
                    key={bang}
                    onClick={() => {
                        const textarea = textareaRef.current;
                        if (textarea) {
                            const cursorPosition = textarea.selectionStart;
                            const bangIndex = Math.max(
                                textarea.value.lastIndexOf(
                                    "!",
                                    cursorPosition - 1,
                                ),
                                textarea.value.lastIndexOf(
                                    "！",
                                    cursorPosition - 1,
                                ),
                            );

                            if (bangIndex !== -1) {
                                let completionText = complete;
                                const cursorIndex = complete.indexOf("|");
                                if (cursorIndex !== -1) {
                                    completionText =
                                        complete.substring(0, cursorIndex) +
                                        complete.substring(cursorIndex + 1);
                                }

                                const newValue =
                                    textarea.value.substring(0, bangIndex + 1) +
                                    completionText +
                                    " " +
                                    textarea.value.substring(cursorPosition);

                                setInputText(newValue);
                                setBangListVisible(false);

                                setTimeout(() => {
                                    textarea.focus();
                                    const newPosition =
                                        bangIndex +
                                        (cursorIndex === -1
                                            ? bang.length + 2
                                            : cursorIndex + 1);
                                    textarea.setSelectionRange(
                                        newPosition,
                                        newPosition,
                                    );
                                });
                            }
                        }
                    }}
                >
                    <span className="completion-bang-tag">{bang}</span>
                    <span className="completion-bang-desc">{desc}</span>
                </div>
            ))}
        </div>
    );
};

export default React.memo(BangCompletionList);
