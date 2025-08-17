import React, { useRef, useEffect } from 'react';
import { ArtifactCollectionItem } from '../../data/ArtifactCollection';

interface ArtifactCompletionListProps {
    artifactListVisible: boolean;
    placement: "top" | "bottom";
    cursorPosition: { bottom: number; left: number; top: number };
    artifacts: ArtifactCollectionItem[];
    selectedArtifactIndex: number;
    textareaRef: React.RefObject<HTMLTextAreaElement | null>;
    setInputText: React.Dispatch<React.SetStateAction<string>>;
    setArtifactListVisible: React.Dispatch<React.SetStateAction<boolean>>;
    onArtifactSelect?: (artifact: ArtifactCollectionItem) => void;
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
    onArtifactSelect
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
        onArtifactSelect?.(artifact);
    };

    if (!artifactListVisible || artifacts.length === 0) {
        return null;
    }

    const listStyle: React.CSSProperties = placement === "top"
        ? {
            position: "absolute",
            top: cursorPosition.top,
            left: cursorPosition.left,
            zIndex: 1000,
        }
        : {
            position: "absolute",
            bottom: cursorPosition.bottom,
            left: cursorPosition.left,
            zIndex: 1000,
        };

    return (
        <div 
            ref={listRef}
            className="completion-list artifact-completion-list" 
            style={listStyle}
        >
            {artifacts.map((artifact, index) => (
                <div
                    key={artifact.id}
                    className={`completion-artifact-container ${
                        index === selectedArtifactIndex ? "selected" : ""
                    }`}
                    onClick={() => handleArtifactClick(artifact)}
                >
                    <div className="artifact-completion-header">
                        <span className="artifact-completion-icon">{artifact.icon}</span>
                        <span className="artifact-completion-name">{artifact.name}</span>
                        <span className="artifact-completion-type">
                            {artifact.artifact_type.toUpperCase()}
                        </span>
                    </div>
                    {artifact.description && (
                        <div className="artifact-completion-description">
                            {artifact.description}
                        </div>
                    )}
                    <div className="artifact-completion-meta">
                        <span className="artifact-completion-uses">
                            {artifact.use_count} 次使用
                        </span>
                        {artifact.tags && (
                            <span className="artifact-completion-tags">
                                {artifact.tags}
                            </span>
                        )}
                    </div>
                </div>
            ))}
        </div>
    );
}