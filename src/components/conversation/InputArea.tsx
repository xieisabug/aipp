/**
 * InputArea 组件 - 中文输入法兼容性说明
 * 
 * ===========================================
 * WebKit2 GTK 中文输入法处理特殊问题及解决方案
 * ===========================================
 * 
 * 问题背景：
 * 在 Tauri 应用中使用 WebKit2 GTK 渲染引擎时，中文输入法（IME）的行为与标准浏览器不同：
 * 
 * 1. **标准浏览器行为**：
 *    - 用户在中文输入法下按回车键确认候选词时，`event.isComposing` 保持为 `true`
 *    - 只有在完全确认输入后，`event.isComposing` 才变为 `false`
 *    - 这样可以正确区分"确认候选词的回车"和"提交消息的回车"
 * 
 * 2. **WebKit2 GTK 的问题**：
 *    - 用户按回车键确认候选词时，`event.isComposing` 错误地变为 `false`
 *    - 这导致确认候选词的回车被误认为是要提交消息的回车
 *    - 用户在中文输入法下无法正常输入，每次确认候选词都会意外触发消息发送
 * 
 * 解决方案：
 * 
 * 1. **手动状态跟踪**：
 *    - 使用 `isComposingRef` 手动跟踪 IME 组合状态
 *    - 不完全依赖 `event.isComposing` 的值
 * 
 * 2. **Composition 事件监听**：
 *    - `onCompositionStart`: 当开始输入中文时设置 `isComposingRef.current = true`
 *    - `onCompositionEnd`: 当完成中文输入时延迟设置 `isComposingRef.current = false`
 * 
 * 3. **延迟状态更新**：
 *    - 在 `compositionend` 事件中使用 100ms 延迟更新状态
 *    - 这确保了键盘事件处理器能正确检测到组合状态
 *    - WebKit2 的事件触发时序有时不稳定，延迟可以解决时序问题
 * 
 * 4. **双重检查机制**：
 *    - 同时检查 `!isComposingRef.current` 和 `!event.isComposing`
 *    - 只有两个条件都满足时才允许回车键触发消息发送
 *    - 这提供了最大的兼容性保障
 * 
 * 使用场景：
 * - 主要影响使用 Tauri + WebKit2 GTK 的桌面应用
 * - 在 Linux 系统上使用中文输入法时尤其重要
 * - 对标准浏览器环境也保持兼容
 * 
 * 注意事项：
 * - 如果未来 WebKit2 GTK 修复了 `isComposing` 的问题，此代码仍然兼容
 * - 延迟时间（100ms）是经过测试的最佳值，不建议随意修改
 * - 此解决方案参考了业界最佳实践（Cherry Studio 等项目的处理方式）
 */

import React, { useRef, useEffect, useState, useCallback } from "react";
import "../../styles/InputArea.css";
import CircleButton from "../CircleButton";
import Add from "../../assets/add.svg?react";
import Stop from "../../assets/stop.svg?react";
import UpArrow from "../../assets/up-arrow.svg?react";
import { FileInfo } from "../../data/Conversation";
import { invoke } from "@tauri-apps/api/core";
import { getCaretCoordinates } from "../../utils/caretCoordinates";
import BangCompletionList from "./BangCompletionList";
import AssistantCompletionList from "./AssistantCompletionList";
import { useFileList } from '../../hooks/useFileList';
import { useAssistantListListener } from '../../hooks/useAssistantListListener';
import PinyinFilter, { AssistantItem, FilteredAssistant } from '../../utils/pinyinFilter';

interface InputAreaProps {
    inputText: string;
    setInputText: React.Dispatch<React.SetStateAction<string>>;
    fileInfoList: FileInfo[] | null;
    handleChooseFile: () => void;
    handlePaste: (e: React.ClipboardEvent<HTMLTextAreaElement>) => void;
    handleDeleteFile: (fileId: number) => void;
    handleSend: () => void;
    aiIsResponsing: boolean;
    placement?: "top" | "bottom";
}

const InputArea: React.FC<InputAreaProps> = React.memo(
    ({
        inputText,
        setInputText,
        fileInfoList,
        handleChooseFile,
        handlePaste,
        handleDeleteFile,
        handleSend,
        aiIsResponsing,
        placement = "bottom",
    }) => {
        // 图片区域的高度
        const IMAGE_AREA_HEIGHT = 80;
        const textareaRef = useRef<HTMLTextAreaElement>(null);

        // WebKit2 GTK 中文输入法兼容性：手动跟踪 IME 组合状态
        // 因为 WebKit2 下 event.isComposing 在确认候选词时会错误地返回 false
        const isComposingRef = useRef(false);
        const [initialHeight, setInitialHeight] = useState<number | null>(null);
        const [bangListVisible, setBangListVisible] = useState<boolean>(false);
        const [bangList, setBangList] = useState<string[]>([]);
        const [originalBangList, setOriginalBangList] = useState<string[]>([]);
        const [cursorPosition, setCursorPosition] = useState<{
            bottom: number;
            left: number;
            top: number;
        }>({ bottom: 0, left: 0, top: 0 });
        const [selectedBangIndex, setSelectedBangIndex] = useState<number>(0);

        // Assistant selection states
        const [assistantListVisible, setAssistantListVisible] = useState<boolean>(false);
        const [assistants, setAssistants] = useState<AssistantItem[]>([]);
        const [filteredAssistants, setFilteredAssistants] = useState<FilteredAssistant[]>([]);
        const [selectedAssistantIndex, setSelectedAssistantIndex] = useState<number>(0);

        const handleOpenFile = (fileId: number) => {
            invoke("open_attachment_with_default_app", { id: fileId });
        }
        const { renderFiles } = useFileList(fileInfoList, handleDeleteFile, handleOpenFile);

        useEffect(() => {
            if (textareaRef.current && !initialHeight) {
                setInitialHeight(textareaRef.current.scrollHeight);
            }
            adjustTextareaHeight();
        }, [inputText, initialHeight, fileInfoList]);

        useEffect(() => {
            invoke<string[]>("get_bang_list").then((bangList) => {
                setBangList(bangList);
                setOriginalBangList(bangList);
            });

            // Load assistants for @ selection
            invoke<AssistantItem[]>("get_assistants").then((assistantList) => {
                setAssistants(assistantList);
                // Initialize with default match info for all assistants
                const initialFiltered: FilteredAssistant[] = assistantList.map(assistant => ({
                    ...assistant,
                    matchType: 'exact' as const,
                    highlightIndices: []
                }));
                setFilteredAssistants(initialFiltered);
            });
        }, []);

        // 监听助手列表变化
        useAssistantListListener({
            onAssistantListChanged: useCallback((assistantList: AssistantItem[]) => {
                setAssistants(assistantList);
                // 重新初始化过滤后的助手列表
                const initialFiltered: FilteredAssistant[] = assistantList.map(assistant => ({
                    ...assistant,
                    matchType: 'exact' as const,
                    highlightIndices: []
                }));
                setFilteredAssistants(initialFiltered);
            }, [])
        });

        useEffect(() => {
            const handleSelectionChange = () => {
                if (textareaRef.current) {
                    const cursorPosition = textareaRef.current.selectionStart;
                    const value = textareaRef.current.value;
                    
                    // Handle @ symbol detection
                    const atIndex = value.lastIndexOf("@", cursorPosition - 1);
                    const bangIndex = Math.max(
                        value.lastIndexOf("!", cursorPosition - 1),
                        value.lastIndexOf("！", cursorPosition - 1),
                    );

                    // Check if @ is closer to cursor than !
                    if (atIndex !== -1 && atIndex > bangIndex) {
                        const atInput = value
                            .substring(atIndex + 1, cursorPosition)
                            .toLowerCase();
                        
                        // Filter assistants using pinyin
                        const filtered = PinyinFilter.filterAssistants(assistants, atInput);
                        
                        if (filtered.length > 0) {
                            setFilteredAssistants(filtered);
                            setSelectedAssistantIndex(0);
                            setAssistantListVisible(true);
                            setBangListVisible(false); // Hide bang list when @ is active

                            const cursorCoords = getCaretCoordinates(
                                textareaRef.current,
                                atIndex + 1,
                            );
                            const rect =
                                textareaRef.current.getBoundingClientRect();
                            const style = window.getComputedStyle(
                                textareaRef.current,
                            );
                            const paddingTop = parseFloat(style.paddingTop);
                            const paddingBottom = parseFloat(
                                style.paddingBottom,
                            );
                            const textareaHeight = parseFloat(style.height);

                            const inputAreaRect = document
                                .querySelector(".input-area")!
                                .getBoundingClientRect();
                            const left =
                                rect.left -
                                inputAreaRect.left +
                                cursorCoords.cursorLeft;

                            if (placement === "top") {
                                const top =
                                    rect.top +
                                    rect.height +
                                    Math.min(
                                        textareaHeight,
                                        cursorCoords.cursorTop,
                                    ) -
                                    paddingTop -
                                    paddingBottom;
                                setCursorPosition({ bottom: 0, left, top });
                            } else {
                                const bottom =
                                    inputAreaRect.top -
                                    rect.top -
                                    cursorCoords.cursorTop +
                                    10 +
                                    (textareaRef.current.scrollHeight -
                                        textareaRef.current.clientHeight);
                                setCursorPosition({ bottom, left, top: 0 });
                            }
                        } else {
                            setAssistantListVisible(false);
                        }
                    } else if (bangIndex !== -1 && bangIndex < cursorPosition) {
                        // Handle ! symbol (existing logic)
                        const bangInput = value
                            .substring(bangIndex + 1, cursorPosition)
                            .toLowerCase();
                        const filteredBangs = originalBangList.filter(
                            ([bang]) =>
                                bang.toLowerCase().startsWith(bangInput),
                        );

                        if (filteredBangs.length > 0) {
                            setBangList(filteredBangs);
                            setSelectedBangIndex(0);
                            setBangListVisible(true);
                            setAssistantListVisible(false); // Hide assistant list when ! is active

                            const cursorCoords = getCaretCoordinates(
                                textareaRef.current,
                                bangIndex + 1,
                            );
                            const rect =
                                textareaRef.current.getBoundingClientRect();
                            const style = window.getComputedStyle(
                                textareaRef.current,
                            );
                            const paddingTop = parseFloat(style.paddingTop);
                            const paddingBottom = parseFloat(
                                style.paddingBottom,
                            );
                            const textareaHeight = parseFloat(style.height);

                            const inputAreaRect = document
                                .querySelector(".input-area")!
                                .getBoundingClientRect();
                            const left =
                                rect.left -
                                inputAreaRect.left +
                                cursorCoords.cursorLeft;

                            if (placement === "top") {
                                const top =
                                    rect.top +
                                    rect.height +
                                    Math.min(
                                        textareaHeight,
                                        cursorCoords.cursorTop,
                                    ) -
                                    paddingTop -
                                    paddingBottom;
                                setCursorPosition({ bottom: 0, left, top });
                            } else {
                                const bottom =
                                    inputAreaRect.top -
                                    rect.top -
                                    cursorCoords.cursorTop +
                                    10 +
                                    (textareaRef.current.scrollHeight -
                                        textareaRef.current.clientHeight);
                                setCursorPosition({ bottom, left, top: 0 });
                            }
                        } else {
                            setBangListVisible(false);
                        }
                    } else {
                        // Hide both lists when neither @ nor ! is active
                        setBangListVisible(false);
                        setAssistantListVisible(false);
                    }
                }
            };

            document.addEventListener("selectionchange", handleSelectionChange);
            return () => {
                document.removeEventListener(
                    "selectionchange",
                    handleSelectionChange,
                );
            };
        }, [originalBangList, assistants, placement]);

        const adjustTextareaHeight = () => {
            const textarea = textareaRef.current;
            if (textarea && initialHeight) {
                textarea.style.height = `${initialHeight}px`;
                const maxHeight = document.documentElement.clientHeight * 0.4;
                const newHeight = Math.min(
                    Math.max(textarea.scrollHeight, initialHeight),
                    maxHeight,
                );
                textarea.style.height = `${newHeight}px`;
                textarea.parentElement!.style.height = `${newHeight + ((fileInfoList?.length && IMAGE_AREA_HEIGHT) || 0)}px`;
            }
        };

        const handleTextareaChange = (
            e: React.ChangeEvent<HTMLTextAreaElement>,
        ) => {
            const newValue = e.target.value;
            const cursorPosition = e.target.selectionStart;
            setInputText(newValue);

            // Check for @ symbol first
            const atIndex = newValue.lastIndexOf("@", cursorPosition - 1);
            const bangIndex = Math.max(
                newValue.lastIndexOf("!", cursorPosition - 1),
                newValue.lastIndexOf("！", cursorPosition - 1),
            );

            // Check if @ is closer to cursor than !
            if (atIndex !== -1 && atIndex > bangIndex) {
                const atInput = newValue
                    .substring(atIndex + 1, cursorPosition)
                    .toLowerCase();

                // Filter assistants using pinyin
                const filtered = PinyinFilter.filterAssistants(assistants, atInput);

                if (filtered.length > 0) {
                    setFilteredAssistants(filtered);
                    setSelectedAssistantIndex(0);
                    setAssistantListVisible(true);
                    setBangListVisible(false); // Hide bang list when @ is active

                    // Update cursor position
                    const textarea = e.target;
                    const cursorCoords = getCaretCoordinates(
                        textarea,
                        cursorPosition,
                    );
                    const rect = textarea.getBoundingClientRect();
                    const style = window.getComputedStyle(textarea);
                    const paddingTop = parseFloat(style.paddingTop);
                    const paddingBottom = parseFloat(style.paddingBottom);
                    const textareaHeight = parseFloat(style.height);
                    const inputAreaRect = document
                        .querySelector(".input-area")!
                        .getBoundingClientRect();

                    const left =
                        rect.left -
                        inputAreaRect.left +
                        cursorCoords.cursorLeft;

                    if (placement === "top") {
                        const top =
                            rect.top +
                            rect.height +
                            Math.min(textareaHeight, cursorCoords.cursorTop) -
                            paddingTop -
                            paddingBottom;

                        setCursorPosition({ bottom: 0, left, top });
                    } else {
                        const bottom =
                            inputAreaRect.top -
                            rect.top -
                            cursorCoords.cursorTop +
                            10 +
                            (textarea.scrollHeight - textarea.clientHeight);
                        setCursorPosition({ bottom, left, top: 0 });
                    }
                } else {
                    setAssistantListVisible(false);
                }
            } else if (bangIndex !== -1 && bangIndex < cursorPosition) {
                // Handle ! symbol (existing logic)
                const bangInput = newValue
                    .substring(bangIndex + 1, cursorPosition)
                    .toLowerCase();
                const filteredBangs = originalBangList.filter(([bang]) =>
                    bang.toLowerCase().startsWith(bangInput),
                );

                if (filteredBangs.length > 0) {
                    setBangList(filteredBangs);
                    setSelectedBangIndex(0);
                    setBangListVisible(true);
                    setAssistantListVisible(false); // Hide assistant list when ! is active

                    // Update cursor position
                    const textarea = e.target;
                    const cursorCoords = getCaretCoordinates(
                        textarea,
                        cursorPosition,
                    );
                    const rect = textarea.getBoundingClientRect();
                    const style = window.getComputedStyle(textarea);
                    const paddingTop = parseFloat(style.paddingTop);
                    const paddingBottom = parseFloat(style.paddingBottom);
                    const textareaHeight = parseFloat(style.height);
                    const inputAreaRect = document
                        .querySelector(".input-area")!
                        .getBoundingClientRect();

                    const left =
                        rect.left -
                        inputAreaRect.left +
                        cursorCoords.cursorLeft;

                    if (placement === "top") {
                        const top =
                            rect.top +
                            rect.height +
                            Math.min(textareaHeight, cursorCoords.cursorTop) -
                            paddingTop -
                            paddingBottom;

                        setCursorPosition({ bottom: 0, left, top });
                    } else {
                        const bottom =
                            inputAreaRect.top -
                            rect.top -
                            cursorCoords.cursorTop +
                            10 +
                            (textarea.scrollHeight - textarea.clientHeight);
                        setCursorPosition({ bottom, left, top: 0 });
                    }
                } else {
                    setBangListVisible(false);
                }
            } else {
                // Hide both lists when neither @ nor ! is active
                setBangListVisible(false);
                setAssistantListVisible(false);
            }
        };

        const handleKeyDownWithBang = (
            e: React.KeyboardEvent<HTMLTextAreaElement>,
        ) => {
            // WebKit2 GTK 中文输入法兼容性：双重检查 IME 状态
            // 1. !isComposingRef.current: 手动跟踪的状态（更可靠）
            // 2. !e.nativeEvent.isComposing: 原生 API 状态（作为补充）
            // 只有两个条件都满足时才认为不在 IME 组合状态，可以安全地处理回车键
            const isEnterPressed = e.key === "Enter" && !isComposingRef.current && !e.nativeEvent.isComposing;

            if (isEnterPressed) {
                if (e.shiftKey) {
                    // Shift + Enter for new line
                    return;
                } else if (assistantListVisible) {
                    // Select assistant - 阻止表单提交
                    e.preventDefault();
                    const selectedAssistant = filteredAssistants[selectedAssistantIndex];
                    const textarea = e.currentTarget as HTMLTextAreaElement;
                    const cursorPosition = textarea.selectionStart;
                    const atIndex = textarea.value.lastIndexOf("@", cursorPosition - 1);

                    if (atIndex !== -1) {
                        const beforeAt = textarea.value.substring(0, atIndex);
                        const afterAt = textarea.value.substring(cursorPosition);
                        setInputText(beforeAt + `@${selectedAssistant.name} ` + afterAt);

                        // 设置光标位置
                        setTimeout(() => {
                            const newPosition = atIndex + selectedAssistant.name.length + 2;
                            textarea.setSelectionRange(newPosition, newPosition);
                        }, 0);
                    }
                    setAssistantListVisible(false);
                } else if (bangListVisible) {
                    // Select bang - 阻止表单提交
                    e.preventDefault();
                    const selectedBang = bangList[selectedBangIndex];
                    let complete = selectedBang[1];
                    const textarea = e.currentTarget as HTMLTextAreaElement;
                    const cursorPosition = textarea.selectionStart;
                    const bangIndex = Math.max(
                        textarea.value.lastIndexOf("!", cursorPosition - 1),
                        textarea.value.lastIndexOf("！", cursorPosition - 1),
                    );

                    if (bangIndex !== -1) {
                        // 找到complete中的|的位置
                        const cursorIndex = complete.indexOf("|");
                        // 如果有|，则将光标移动到|的位置，并且移除|
                        if (cursorIndex !== -1) {
                            complete =
                                complete.substring(0, cursorIndex) +
                                complete.substring(cursorIndex + 1);
                        }

                        const beforeBang = textarea.value.substring(
                            0,
                            bangIndex,
                        );
                        const afterBang =
                            textarea.value.substring(cursorPosition);
                        setInputText(
                            beforeBang + "!" + complete + " " + afterBang,
                        );

                        // 设置光标位置
                        setTimeout(() => {
                            const newPosition =
                                bangIndex +
                                (cursorIndex === -1
                                    ? selectedBang[0].length + 2
                                    : cursorIndex + 1);
                            textarea.setSelectionRange(
                                newPosition,
                                newPosition,
                            );
                        }, 0);
                    }
                    setBangListVisible(false);
                } else {
                    // Enter for submit (only if not in IME composition)
                    e.preventDefault();
                    handleSend();
                }
            } else if (e.key === "Tab" && assistantListVisible) {
                // Select assistant
                e.preventDefault();
                const selectedAssistant = filteredAssistants[selectedAssistantIndex];
                const textarea = e.currentTarget as HTMLTextAreaElement;
                const cursorPosition = textarea.selectionStart;
                const atIndex = textarea.value.lastIndexOf("@", cursorPosition - 1);

                if (atIndex !== -1) {
                    const beforeAt = textarea.value.substring(0, atIndex);
                    const afterAt = textarea.value.substring(cursorPosition);
                    setInputText(beforeAt + `@${selectedAssistant.name} ` + afterAt);

                    // 设置光标位置
                    setTimeout(() => {
                        const newPosition = atIndex + selectedAssistant.name.length + 2;
                        textarea.setSelectionRange(newPosition, newPosition);
                    }, 0);
                }
                setAssistantListVisible(false);
            } else if (e.key === "Tab" && bangListVisible) {
                // Select bang
                e.preventDefault();
                const selectedBang = bangList[selectedBangIndex];
                let complete = selectedBang[1];
                const textarea = e.currentTarget as HTMLTextAreaElement;
                const cursorPosition = textarea.selectionStart;
                const bangIndex = Math.max(
                    textarea.value.lastIndexOf("!", cursorPosition - 1),
                    textarea.value.lastIndexOf("！", cursorPosition - 1),
                );

                if (bangIndex !== -1) {
                    // 找到complete中的|的位置
                    const cursorIndex = complete.indexOf("|");
                    // 如果有|，则将光标移动到|的位置，并且移除|
                    if (cursorIndex !== -1) {
                        complete =
                            complete.substring(0, cursorIndex) +
                            complete.substring(cursorIndex + 1);
                    }

                    const beforeBang = textarea.value.substring(0, bangIndex);
                    const afterBang = textarea.value.substring(cursorPosition);
                    setInputText(beforeBang + "!" + complete + " " + afterBang);

                    // 设置光标位置
                    setTimeout(() => {
                        const newPosition =
                            bangIndex +
                            (cursorIndex === -1
                                ? selectedBang[0].length + 2
                                : cursorIndex + 1);
                        textarea.setSelectionRange(newPosition, newPosition);
                    }, 0);
                }
                setBangListVisible(false);
            } else if (e.key === "ArrowUp" && assistantListVisible) {
                e.preventDefault();
                setSelectedAssistantIndex((prevIndex) =>
                    prevIndex > 0 ? prevIndex - 1 : filteredAssistants.length - 1,
                );
            } else if (e.key === "ArrowDown" && assistantListVisible) {
                e.preventDefault();
                setSelectedAssistantIndex((prevIndex) =>
                    prevIndex < filteredAssistants.length - 1 ? prevIndex + 1 : 0,
                );
            } else if (e.key === "ArrowUp" && bangListVisible) {
                e.preventDefault();
                setSelectedBangIndex((prevIndex) =>
                    prevIndex > 0 ? prevIndex - 1 : bangList.length - 1,
                );
            } else if (e.key === "ArrowDown" && bangListVisible) {
                e.preventDefault();
                setSelectedBangIndex((prevIndex) =>
                    prevIndex < bangList.length - 1 ? prevIndex + 1 : 0,
                );
            } else if (e.key === "Escape") {
                e.preventDefault();
                setBangListVisible(false);
                setAssistantListVisible(false);
            }
        };


        function scrollToSelectedBang() {
            const selectedBangElement = document.querySelector(
                ".completion-bang-container.selected",
            );
            if (selectedBangElement) {
                const parentElement = selectedBangElement.parentElement;
                if (parentElement) {
                    const parentRect = parentElement.getBoundingClientRect();
                    const selectedRect =
                        selectedBangElement.getBoundingClientRect();

                    if (selectedRect.top < parentRect.top) {
                        parentElement.scrollTop -=
                            parentRect.top - selectedRect.top;
                    } else if (selectedRect.bottom > parentRect.bottom) {
                        parentElement.scrollTop +=
                            selectedRect.bottom - parentRect.bottom;
                    }
                }
            }
        }
        useEffect(() => {
            scrollToSelectedBang();
        }, [selectedBangIndex]);

        const handleImageContainerClick = useCallback(() => {
            textareaRef.current?.focus();
        }, [textareaRef]);

        return (
            <div className={`input-area ${placement}`}>
                <div className="input-area-textarea-container">
                    <div className="input-area-img-container" onClick={handleImageContainerClick}>
                        {renderFiles()}
                    </div>
                    <textarea
                        ref={textareaRef}
                        className="input-area-textarea"
                        rows={1}
                        value={inputText}
                        onChange={handleTextareaChange}
                        onKeyDown={handleKeyDownWithBang}
                        onPaste={handlePaste}
                        onCompositionStart={() => {
                            // WebKit2 GTK 中文输入法兼容性：标记开始 IME 组合
                            isComposingRef.current = true;
                        }}
                        onCompositionEnd={() => {
                            // WebKit2 GTK 中文输入法兼容性：延迟标记结束 IME 组合
                            // 使用 100ms 延迟是因为 WebKit2 的事件时序不稳定
                            // 确保键盘事件处理器能正确检测到组合状态
                            setTimeout(() => {
                                isComposingRef.current = false;
                            }, 100);
                        }}
                    />
                </div>

                <CircleButton
                    onClick={handleChooseFile}
                    icon={<Add className="fill-foreground" />}
                    className={`input-area-add-button ${placement}`}
                />
                <CircleButton
                    size={placement === "bottom" ? "large" : "medium"}
                    onClick={handleSend}
                    icon={
                        aiIsResponsing ? (
                            <Stop width={20} height={20} className="fill-primary-foreground" />
                        ) : (
                            <UpArrow width={20} height={20} className="fill-primary-foreground" />
                        )
                    }
                    primary
                    className={`input-area-send-button ${placement}`}
                />

                <BangCompletionList
                    bangListVisible={bangListVisible}
                    placement={placement}
                    cursorPosition={cursorPosition}
                    bangList={bangList}
                    selectedBangIndex={selectedBangIndex}
                    textareaRef={textareaRef}
                    setInputText={setInputText}
                    setBangListVisible={setBangListVisible}
                />

                <AssistantCompletionList
                    assistantListVisible={assistantListVisible}
                    placement={placement}
                    cursorPosition={cursorPosition}
                    assistants={filteredAssistants}
                    selectedAssistantIndex={selectedAssistantIndex}
                    textareaRef={textareaRef}
                    setInputText={setInputText}
                    setAssistantListVisible={setAssistantListVisible}
                />
            </div>
        );
    },
);

export default InputArea;