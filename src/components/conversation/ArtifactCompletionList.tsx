import React, { useRef, useEffect } from "react";
import { ArtifactCollectionItem } from "../../data/ArtifactCollection";
import { formatIconDisplay } from "@/utils/emojiUtils";

interface ArtifactCompletionListProps {
    artifactListVisible: boolean;
    placement: "top" | "bottom";
    cursorPosition: { bottom: number; left: number; top: number };
    artifacts: ArtifactCollectionItem[];
    selectedArtifactIndex: number;
    textareaRef: React.RefObject<HTMLTextAreaElement | null>;
    setInputText: React.Dispatch<React.SetStateAction<string>>;
    setArtifactListVisible: React.Dispatch<React.SetStateAction<boolean>>;
    onArtifactSelect?: (artifact: ArtifactCollectionItem, action: "complete" | "open") => void;
}

export default function ArtifactCompletionList({
    artifactListVisible,
    placement,
    cursorPosition,
    artifacts,
    selectedArtifactIndex,
    textareaRef,
    setInputText,
    setArtifactListVisible,
    onArtifactSelect,
}: ArtifactCompletionListProps) {
    const listRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (artifactListVisible && selectedArtifactIndex >= 0) {
            scrollToSelectedArtifact();
        }
    }, [selectedArtifactIndex, artifactListVisible]);

    const scrollToSelectedArtifact = () => {
        const selectedArtifactElement = document.querySelector(
            ".completion-artifact-container.selected"
        ) as HTMLElement;

        if (selectedArtifactElement && listRef.current) {
            const parentRect = listRef.current.getBoundingClientRect();
            const selectedRect = selectedArtifactElement.getBoundingClientRect();

            if (selectedRect.top < parentRect.top) {
                listRef.current.scrollTop -= parentRect.top - selectedRect.top;
            } else if (selectedRect.bottom > parentRect.bottom) {
                listRef.current.scrollTop += selectedRect.bottom - parentRect.bottom;
            }
        }
    };

    const handleArtifactClick = (artifact: ArtifactCollectionItem) => {
        const textarea = textareaRef.current;
        if (!textarea) return;

        const cursorPosition = textarea.selectionStart;
        const hashIndex = textarea.value.lastIndexOf("#", cursorPosition - 1);

        if (hashIndex !== -1) {
            const beforeHash = textarea.value.substring(0, hashIndex);
            const afterHash = textarea.value.substring(cursorPosition);
            setInputText(beforeHash + `#${artifact.name} ` + afterHash);

            // Set cursor position after the inserted text
            setTimeout(() => {
                const newPosition = hashIndex + artifact.name.length + 2;
                textarea.setSelectionRange(newPosition, newPosition);
                textarea.focus();
            }, 0);
        }

        setArtifactListVisible(false);
        onArtifactSelect?.(artifact, "complete");
    };

    if (!artifactListVisible || artifacts.length === 0) {
        return null;
    }

    return (
        <div
            ref={listRef}
            className={`completion-list artifact-completion-list`}
            style={{
                ...(placement === "top" ? { top: cursorPosition.top } : { bottom: cursorPosition.bottom }),
                left: cursorPosition.left,
            }}
        >
            <div className="artifact-completion-hint">Tab 补全名称 • Enter 打开</div>
            {artifacts.map((artifact, index) => (
                <div
                    key={artifact.id}
                    className={`completion-artifact-container ${index === selectedArtifactIndex ? "selected" : ""}`}
                    onClick={() => handleArtifactClick(artifact)}
                >
                    <div className="artifact-completion-header">
                        <span className="artifact-completion-icon">
                            {(() => {
                                const iconDisplay = formatIconDisplay(artifact.icon);
                                return iconDisplay.isImage ? (
                                    <img
                                        src={iconDisplay.display}
                                        alt={`Icon for ${artifact.name}`}
                                        className="w-5 h-5 object-cover"
                                    />
                                ) : (
                                    iconDisplay.display
                                );
                            })()}
                        </span>
                        <span className="artifact-completion-name">{artifact.name}</span>
                        <span className="artifact-completion-type">{artifact.artifact_type}</span>
                    </div>
                    {artifact.description && (
                        <div className="artifact-completion-description">{artifact.description}</div>
                    )}
                    <div className="artifact-completion-meta">
                        <span className="artifact-completion-uses">{artifact.use_count} 次使用</span>
                        {artifact.tags && <span className="artifact-completion-tags">{artifact.tags}</span>}
                    </div>
                </div>
            ))}
        </div>
    );
}
