import React, { useEffect, useRef } from "react";
import { FilteredAssistant } from "../../utils/pinyinFilter";

interface AssistantCompletionListProps {
    assistantListVisible: boolean;
    placement: "top" | "bottom";
    cursorPosition: {
        bottom: number;
        left: number;
        top: number;
    };
    assistants: FilteredAssistant[];
    selectedAssistantIndex: number;
    textareaRef: React.RefObject<HTMLTextAreaElement | null>;
    setInputText: React.Dispatch<React.SetStateAction<string>>;
    setAssistantListVisible: React.Dispatch<React.SetStateAction<boolean>>;
}

const AssistantCompletionList: React.FC<AssistantCompletionListProps> = ({
    assistantListVisible,
    placement,
    cursorPosition,
    assistants,
    selectedAssistantIndex,
    textareaRef,
    setInputText,
    setAssistantListVisible,
}) => {
    const listRef = useRef<HTMLDivElement>(null);

    const scrollToSelectedAssistant = () => {
        const parentElement = listRef.current;
        if (parentElement && selectedAssistantIndex >= 0) {
            const selectedElement = parentElement.querySelector(
                `.assistant-completion-item:nth-child(${selectedAssistantIndex + 1})`
            ) as HTMLElement;

            if (selectedElement) {
                const parentRect = parentElement.getBoundingClientRect();
                const selectedRect = selectedElement.getBoundingClientRect();

                if (selectedRect.top < parentRect.top) {
                    parentElement.scrollTop -= parentRect.top - selectedRect.top;
                } else if (selectedRect.bottom > parentRect.bottom) {
                    parentElement.scrollTop += selectedRect.bottom - parentRect.bottom;
                }
            }
        }
    };

    useEffect(() => {
        scrollToSelectedAssistant();
    }, [selectedAssistantIndex]);

    const handleAssistantSelect = (assistant: FilteredAssistant) => {
        if (!textareaRef.current) return;

        const textarea = textareaRef.current;
        const cursorPosition = textarea.selectionStart;
        const value = textarea.value;

        // Find the @ symbol position
        const atIndex = Math.max(value.lastIndexOf("@", cursorPosition - 1));

        if (atIndex !== -1) {
            // Get the text after @ symbol (not used in current logic but kept for potential future use)

            const beforeAt = value.substring(0, atIndex);
            const afterCursor = value.substring(cursorPosition);

            // Replace @ + search text with just @ + assistant name
            setInputText(beforeAt + "@" + assistant.name + " " + afterCursor.trimStart());

            // Set cursor position after the assistant name
            setTimeout(() => {
                const newPosition = atIndex + 1 + assistant.name.length + 1;
                textarea.setSelectionRange(newPosition, newPosition);
                textarea.focus();
            }, 0);
        }

        setAssistantListVisible(false);
    };

    const renderHighlightedText = (text: string, highlightIndices: number[]) => {
        if (highlightIndices.length === 0) {
            return text;
        }

        const chars = text.split("");
        return chars.map((char, index) => {
            const isHighlighted = highlightIndices.includes(index);
            return (
                <span key={index} className={isHighlighted ? "font-bold text-primary" : ""}>
                    {char}
                </span>
            );
        });
    };

    const getMatchTypeLabel = (matchType: string) => {
        switch (matchType) {
            case "exact":
                return null;
            case "pinyin":
                return <span className="assistant-completion-match-type pinyin">拼音</span>;
            case "initial":
                return (
                    <span className="assistant-completion-match-type initial">首字母</span>
                );
            default:
                return null;
        }
    };

    if (!assistantListVisible || assistants.length === 0) {
        return null;
    }

    return (
        <div
            ref={listRef}
            className="assistant-completion-list"
            style={{
                ...(placement === "top"
                    ? { top: cursorPosition.top }
                    : { bottom: cursorPosition.bottom }),
                left: cursorPosition.left,
            }}
        >
            {assistants.map((assistant, index) => (
                <div
                    key={assistant.id}
                    className={`assistant-completion-item ${
                        index === selectedAssistantIndex ? "selected" : ""
                    }`}
                    onClick={() => handleAssistantSelect(assistant)}
                    onMouseEnter={() => {
                        // We could update selected index on hover if needed
                    }}
                >
                    <div className="assistant-completion-content">
                        <div className="assistant-completion-info">
                            <div className="assistant-completion-name">
                                {renderHighlightedText(assistant.name, assistant.highlightIndices)}
                            </div>
                            {assistant.description && (
                                <div className="assistant-completion-description">
                                    {assistant.description}
                                </div>
                            )}
                        </div>
                        {getMatchTypeLabel(assistant.matchType)}
                    </div>
                </div>
            ))}
        </div>
    );
};

export default AssistantCompletionList;
