import React, { useEffect, useRef } from 'react';
import { FilteredAssistant } from '../../utils/pinyinFilter';

interface AssistantCompletionListProps {
  assistantListVisible: boolean;
  placement: 'top' | 'bottom';
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
          parentElement.scrollTop -=
            parentRect.top - selectedRect.top;
        } else if (selectedRect.bottom > parentRect.bottom) {
          parentElement.scrollTop +=
            selectedRect.bottom - parentRect.bottom;
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
    const atIndex = Math.max(
      value.lastIndexOf('@', cursorPosition - 1),
    );

    if (atIndex !== -1) {
      // Get the text after @ symbol (not used in current logic but kept for potential future use)
      
      const beforeAt = value.substring(0, atIndex);
      const afterCursor = value.substring(cursorPosition);
      
      // Replace @ + search text with just @ + assistant name
      setInputText(beforeAt + '@' + assistant.name + ' ' + afterCursor.trimStart());

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

    const chars = text.split('');
    return chars.map((char, index) => {
      const isHighlighted = highlightIndices.includes(index);
      return (
        <span
          key={index}
          className={isHighlighted ? 'font-bold text-indigo-600' : ''}
        >
          {char}
        </span>
      );
    });
  };

  const getMatchTypeLabel = (matchType: string) => {
    switch (matchType) {
      case 'exact':
        return null;
      case 'pinyin':
        return (
          <span className="text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded-full ml-2">
            拼音
          </span>
        );
      case 'initial':
        return (
          <span className="text-xs bg-green-100 text-green-700 px-2 py-1 rounded-full ml-2">
            首字母
          </span>
        );
      default:
        return null;
    }
  };

  if (!assistantListVisible || assistants.length === 0) {
    return null;
  }

  const style = placement === 'top' 
    ? { 
        top: `${cursorPosition.top}px`, 
        left: `${cursorPosition.left}px`
      }
    : { 
        bottom: `${cursorPosition.bottom}px`, 
        left: `${cursorPosition.left}px` 
      };

  return (
    <div
      ref={listRef}
      className="assistant-completion-list absolute z-50 bg-white border border-gray-200 rounded-lg shadow-lg max-w-xs max-h-64 overflow-y-auto"
      style={style}
    >
      {assistants.map((assistant, index) => (
        <div
          key={assistant.id}
          className={`assistant-completion-item px-3 py-2 cursor-pointer border-b border-gray-100 last:border-b-0 ${
            index === selectedAssistantIndex ? 'bg-gray-100' : 'hover:bg-gray-50'
          }`}
          onClick={() => handleAssistantSelect(assistant)}
          onMouseEnter={() => {
            // We could update selected index on hover if needed
          }}
        >
          <div className="flex items-center justify-between">
            <div className="flex-1 min-w-0">
              <div className="text-sm font-medium text-gray-900 truncate">
                {renderHighlightedText(assistant.name, assistant.highlightIndices)}
              </div>
              {assistant.description && (
                <div className="text-xs text-gray-500 truncate mt-1">
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