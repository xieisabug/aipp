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

import React, { useRef, useEffect, useState, useCallback, forwardRef, useImperativeHandle } from "react";
import "../../styles/InputArea.css";
import CircleButton from "../CircleButton";
import Add from "../../assets/add.svg?react";
import Stop from "../../assets/stop.svg?react";
import UpArrow from "../../assets/up-arrow.svg?react";
import { FileInfo } from "../../data/Conversation";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCaretCoordinates } from "../../utils/caretCoordinates";
import BangCompletionList from "./BangCompletionList";
import AssistantCompletionList from "./AssistantCompletionList";
import ArtifactCompletionList from "./ArtifactCompletionList";
import { useFileList } from "../../hooks/useFileList";
import { useAssistantListListener } from "../../hooks/useAssistantListListener";
import PinyinFilter, { AssistantItem, FilteredAssistant } from "../../utils/pinyinFilter";
import { ArtifactCollectionItem, FilteredArtifact } from "../../data/ArtifactCollection";

// 暴露给外部的方法接口
export interface InputAreaRef {
    focus: () => void;
}

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
const IMAGE_AREA_HEIGHT = 80;

const InputArea = React.memo(
    forwardRef<InputAreaRef, InputAreaProps>(
        (
            {
                inputText,
                setInputText,
                fileInfoList,
                handleChooseFile,
                handlePaste,
                handleDeleteFile,
                handleSend,
                aiIsResponsing,
                placement = "bottom",
            },
            ref
        ) => {
            // 图片区域的高度
            const textareaRef = useRef<HTMLTextAreaElement>(null);

            // 暴露给外部的方法
            useImperativeHandle(
                ref,
                () => ({
                    focus: () => {
                        textareaRef.current?.focus();
                    },
                }),
                []
            );

            // WebKit2 GTK 中文输入法兼容性：手动跟踪 IME 组合状态
            // 因为 WebKit2 下 event.isComposing 在确认候选词时会错误地返回 false
            const isComposingRef = useRef(false);
            const [initialHeight, setInitialHeight] = useState<number | null>(null);
            const [isFocused, setIsFocused] = useState<boolean>(false);
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

            // Artifact selection states
            const [artifactListVisible, setArtifactListVisible] = useState<boolean>(false);
            const [artifacts, setArtifacts] = useState<ArtifactCollectionItem[]>([]);
            const [filteredArtifacts, setFilteredArtifacts] = useState<FilteredArtifact[]>([]);
            const [selectedArtifactIndex, setSelectedArtifactIndex] = useState<number>(0);

            const handleOpenFile = (fileId: number) => {
                invoke("open_attachment_with_default_app", { id: fileId });
            };
            const { renderFiles } = useFileList(fileInfoList, handleDeleteFile, handleOpenFile);

            useEffect(() => {
                if (textareaRef.current && !initialHeight) {
                    setInitialHeight(textareaRef.current.scrollHeight);
                }
                // Only adjust height when focused or when content changes
                if (isFocused) {
                    adjustTextareaHeight();
                }
            }, [inputText, initialHeight, fileInfoList, isFocused]);

            useEffect(() => {
                invoke<string[]>("get_bang_list").then((bangList) => {
                    setBangList(bangList);
                    setOriginalBangList(bangList);
                });

                // Load assistants for @ selection
                invoke<AssistantItem[]>("get_assistants").then((assistantList) => {
                    setAssistants(assistantList);
                    // Initialize with default match info for all assistants
                    const initialFiltered: FilteredAssistant[] = assistantList.map((assistant) => ({
                        ...assistant,
                        matchType: "exact" as const,
                        highlightIndices: [],
                    }));
                    setFilteredAssistants(initialFiltered);
                });

                // Load artifacts for # selection
                const loadArtifacts = () => {
                    invoke<ArtifactCollectionItem[]>("get_artifacts_for_completion").then((artifactList) => {
                        setArtifacts(artifactList);
                        // Initialize with default match info for all artifacts
                        const initialFiltered: FilteredArtifact[] = artifactList.map((artifact) => ({
                            ...artifact,
                            matchType: "exact" as const,
                            highlightIndices: [],
                        }));
                        setFilteredArtifacts(initialFiltered);
                    });
                };

                loadArtifacts();

                // Listen for artifact collection updates
                const setupArtifactListener = async () => {
                    const unlisten = await listen("artifact-collection-updated", () => {
                        loadArtifacts();
                    });
                    return unlisten;
                };

                let unlistenPromise: Promise<() => void> | null = null;

                try {
                    unlistenPromise = setupArtifactListener();
                } catch (error) {
                    console.warn("Failed to setup artifact listener:", error);
                }

                return () => {
                    if (unlistenPromise) {
                        unlistenPromise.then((unlisten) => unlisten()).catch(console.warn);
                    }
                };
            }, []);

            // 监听助手列表变化
            useAssistantListListener({
                onAssistantListChanged: useCallback((assistantList: AssistantItem[]) => {
                    setAssistants(assistantList);
                    // 重新初始化过滤后的助手列表
                    const initialFiltered: FilteredAssistant[] = assistantList.map((assistant) => ({
                        ...assistant,
                        matchType: "exact" as const,
                        highlightIndices: [],
                    }));
                    setFilteredAssistants(initialFiltered);
                }, []),
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
                            value.lastIndexOf("！", cursorPosition - 1)
                        );
                        const hashIndex = value.lastIndexOf("#", cursorPosition - 1);

                        // Find the most recent symbol
                        const maxIndex = Math.max(atIndex, bangIndex, hashIndex);

                        if (hashIndex !== -1 && hashIndex === maxIndex) {
                            // Handle # symbol for artifacts
                            const hashInput = value.substring(hashIndex + 1, cursorPosition).toLowerCase();

                            // Filter artifacts using pinyin
                            const filtered = PinyinFilter.filterArtifacts(artifacts, hashInput);

                            if (filtered.length > 0) {
                                setFilteredArtifacts(filtered);
                                setSelectedArtifactIndex(0);
                                setArtifactListVisible(true);
                                setBangListVisible(false);
                                setAssistantListVisible(false);

                                const cursorCoords = getCaretCoordinates(textareaRef.current, hashIndex + 1);
                                const rect = textareaRef.current.getBoundingClientRect();
                                const style = window.getComputedStyle(textareaRef.current);
                                const paddingTop = parseFloat(style.paddingTop);
                                const paddingBottom = parseFloat(style.paddingBottom);
                                const textareaHeight = parseFloat(style.height);

                                const inputAreaRect = document.querySelector(".input-area")!.getBoundingClientRect();
                                const left = rect.left - inputAreaRect.left + cursorCoords.cursorLeft;

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
                                        (textareaRef.current.scrollHeight - textareaRef.current.clientHeight);
                                    setCursorPosition({ bottom, left, top: 0 });
                                }
                            } else {
                                setArtifactListVisible(false);
                            }
                        } else if (atIndex !== -1 && atIndex === maxIndex) {
                            // Handle @ symbol for assistants
                            const atInput = value.substring(atIndex + 1, cursorPosition).toLowerCase();

                            // Filter assistants using pinyin
                            const filtered = PinyinFilter.filterAssistants(assistants, atInput);

                            if (filtered.length > 0) {
                                setFilteredAssistants(filtered);
                                setSelectedAssistantIndex(0);
                                setAssistantListVisible(true);
                                setBangListVisible(false);
                                setArtifactListVisible(false);

                                const cursorCoords = getCaretCoordinates(textareaRef.current, atIndex + 1);
                                const rect = textareaRef.current.getBoundingClientRect();
                                const style = window.getComputedStyle(textareaRef.current);
                                const paddingTop = parseFloat(style.paddingTop);
                                const paddingBottom = parseFloat(style.paddingBottom);
                                const textareaHeight = parseFloat(style.height);

                                const inputAreaRect = document.querySelector(".input-area")!.getBoundingClientRect();
                                const left = rect.left - inputAreaRect.left + cursorCoords.cursorLeft;

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
                                        (textareaRef.current.scrollHeight - textareaRef.current.clientHeight);
                                    setCursorPosition({ bottom, left, top: 0 });
                                }
                            } else {
                                setAssistantListVisible(false);
                            }
                        } else if (bangIndex !== -1 && bangIndex === maxIndex) {
                            // Handle ! symbol (existing logic)
                            const bangInput = value.substring(bangIndex + 1, cursorPosition).toLowerCase();
                            const filteredBangs = originalBangList.filter(([bang]) =>
                                bang.toLowerCase().startsWith(bangInput)
                            );

                            if (filteredBangs.length > 0) {
                                setBangList(filteredBangs);
                                setSelectedBangIndex(0);
                                setBangListVisible(true);
                                setAssistantListVisible(false);
                                setArtifactListVisible(false);

                                const cursorCoords = getCaretCoordinates(textareaRef.current, bangIndex + 1);
                                const rect = textareaRef.current.getBoundingClientRect();
                                const style = window.getComputedStyle(textareaRef.current);
                                const paddingTop = parseFloat(style.paddingTop);
                                const paddingBottom = parseFloat(style.paddingBottom);
                                const textareaHeight = parseFloat(style.height);

                                const inputAreaRect = document.querySelector(".input-area")!.getBoundingClientRect();
                                const left = rect.left - inputAreaRect.left + cursorCoords.cursorLeft;

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
                                        (textareaRef.current.scrollHeight - textareaRef.current.clientHeight);
                                    setCursorPosition({ bottom, left, top: 0 });
                                }
                            } else {
                                setBangListVisible(false);
                            }
                        } else {
                            // Hide all lists when no trigger symbol is active
                            setBangListVisible(false);
                            setAssistantListVisible(false);
                            setArtifactListVisible(false);
                        }
                    }
                };

                document.addEventListener("selectionchange", handleSelectionChange);
                return () => {
                    document.removeEventListener("selectionchange", handleSelectionChange);
                };
            }, [originalBangList, assistants, artifacts, placement]);

            const adjustTextareaHeight = () => {
                const textarea = textareaRef.current;
                if (textarea && initialHeight) {
                    textarea.style.height = `${initialHeight}px`;
                    const maxHeight = document.documentElement.clientHeight * 0.35;
                    const newHeight = Math.min(Math.max(textarea.scrollHeight, initialHeight), maxHeight);
                    textarea.style.height = `${newHeight}px`;
                    textarea.parentElement!.style.height = `${newHeight + ((fileInfoList?.length && IMAGE_AREA_HEIGHT) || 0)}px`;
                }
            };

            const restoreInitialHeight = () => {
                const textarea = textareaRef.current;
                if (textarea && initialHeight) {
                    textarea.style.height = `${initialHeight}px`;
                    textarea.parentElement!.style.height = `${initialHeight + ((fileInfoList?.length && IMAGE_AREA_HEIGHT) || 0)}px`;
                }
            };

            const handleTextareaChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
                const newValue = e.target.value;
                const cursorPosition = e.target.selectionStart;
                setInputText(newValue);

                // Check for symbols
                const atIndex = newValue.lastIndexOf("@", cursorPosition - 1);
                const bangIndex = Math.max(
                    newValue.lastIndexOf("!", cursorPosition - 1),
                    newValue.lastIndexOf("！", cursorPosition - 1)
                );
                const hashIndex = newValue.lastIndexOf("#", cursorPosition - 1);

                // Find the most recent symbol
                const maxIndex = Math.max(atIndex, bangIndex, hashIndex);

                if (hashIndex !== -1 && hashIndex === maxIndex) {
                    // Handle # symbol for artifacts
                    const hashInput = newValue.substring(hashIndex + 1, cursorPosition).toLowerCase();

                    // Filter artifacts using pinyin
                    const filtered = PinyinFilter.filterArtifacts(artifacts, hashInput);

                    if (filtered.length > 0) {
                        setFilteredArtifacts(filtered);
                        setSelectedArtifactIndex(0);
                        setArtifactListVisible(true);
                        setBangListVisible(false);
                        setAssistantListVisible(false);

                        // Update cursor position
                        const textarea = e.target;
                        const cursorCoords = getCaretCoordinates(textarea, cursorPosition);
                        const rect = textarea.getBoundingClientRect();
                        const style = window.getComputedStyle(textarea);
                        const paddingTop = parseFloat(style.paddingTop);
                        const paddingBottom = parseFloat(style.paddingBottom);
                        const textareaHeight = parseFloat(style.height);
                        const inputAreaRect = document.querySelector(".input-area")!.getBoundingClientRect();

                        const left = rect.left - inputAreaRect.left + cursorCoords.cursorLeft;

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
                        setArtifactListVisible(false);
                    }
                } else if (atIndex !== -1 && atIndex === maxIndex) {
                    // Handle @ symbol for assistants
                    const atInput = newValue.substring(atIndex + 1, cursorPosition).toLowerCase();

                    // Filter assistants using pinyin
                    const filtered = PinyinFilter.filterAssistants(assistants, atInput);

                    if (filtered.length > 0) {
                        setFilteredAssistants(filtered);
                        setSelectedAssistantIndex(0);
                        setAssistantListVisible(true);
                        setBangListVisible(false);
                        setArtifactListVisible(false);

                        // Update cursor position
                        const textarea = e.target;
                        const cursorCoords = getCaretCoordinates(textarea, cursorPosition);
                        const rect = textarea.getBoundingClientRect();
                        const style = window.getComputedStyle(textarea);
                        const paddingTop = parseFloat(style.paddingTop);
                        const paddingBottom = parseFloat(style.paddingBottom);
                        const textareaHeight = parseFloat(style.height);
                        const inputAreaRect = document.querySelector(".input-area")!.getBoundingClientRect();

                        const left = rect.left - inputAreaRect.left + cursorCoords.cursorLeft;

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
                } else if (bangIndex !== -1 && bangIndex === maxIndex) {
                    // Handle ! symbol (existing logic)
                    const bangInput = newValue.substring(bangIndex + 1, cursorPosition).toLowerCase();
                    const filteredBangs = originalBangList.filter(([bang]) => bang.toLowerCase().startsWith(bangInput));

                    if (filteredBangs.length > 0) {
                        setBangList(filteredBangs);
                        setSelectedBangIndex(0);
                        setBangListVisible(true);
                        setAssistantListVisible(false);
                        setArtifactListVisible(false);

                        // Update cursor position
                        const textarea = e.target;
                        const cursorCoords = getCaretCoordinates(textarea, cursorPosition);
                        const rect = textarea.getBoundingClientRect();
                        const style = window.getComputedStyle(textarea);
                        const paddingTop = parseFloat(style.paddingTop);
                        const paddingBottom = parseFloat(style.paddingBottom);
                        const textareaHeight = parseFloat(style.height);
                        const inputAreaRect = document.querySelector(".input-area")!.getBoundingClientRect();

                        const left = rect.left - inputAreaRect.left + cursorCoords.cursorLeft;

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
                    // Hide all lists when no trigger symbol is active
                    setBangListVisible(false);
                    setAssistantListVisible(false);
                    setArtifactListVisible(false);
                }
            };

            const handleKeyDownWithBang = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
                // WebKit2 GTK 中文输入法兼容性：双重检查 IME 状态
                // 1. !isComposingRef.current: 手动跟踪的状态（更可靠）
                // 2. !e.nativeEvent.isComposing: 原生 API 状态（作为补充）
                // 只有两个条件都满足时才认为不在 IME 组合状态，可以安全地处理回车键
                const isEnterPressed = e.key === "Enter" && !isComposingRef.current && !e.nativeEvent.isComposing;

                if (isEnterPressed) {
                    if (e.shiftKey) {
                        // Shift + Enter for new line
                        return;
                    } else if (artifactListVisible) {
                        // Open artifact - 阻止表单提交
                        e.preventDefault();
                        const selectedArtifact = filteredArtifacts[selectedArtifactIndex];
                        const textarea = e.currentTarget as HTMLTextAreaElement;
                        const cursorPosition = textarea.selectionStart;
                        const hashIndex = textarea.value.lastIndexOf("#", cursorPosition - 1);

                        if (hashIndex !== -1) {
                            // Clear the #artifact_name from input
                            const beforeHash = textarea.value.substring(0, hashIndex);
                            const afterHash = textarea.value.substring(cursorPosition);
                            setInputText(beforeHash + afterHash);

                            // Set cursor position at the hash position
                            setTimeout(() => {
                                textarea.setSelectionRange(hashIndex, hashIndex);
                            }, 0);
                        }
                        setArtifactListVisible(false);

                        // Open the artifact
                        invoke("open_artifact_window", { artifactId: selectedArtifact.id }).catch((error) => {
                            console.error("Failed to open artifact:", error);
                        });
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
                            textarea.value.lastIndexOf("！", cursorPosition - 1)
                        );

                        if (bangIndex !== -1) {
                            // 找到complete中的|的位置
                            const cursorIndex = complete.indexOf("|");
                            // 如果有|，则将光标移动到|的位置，并且移除|
                            if (cursorIndex !== -1) {
                                complete = complete.substring(0, cursorIndex) + complete.substring(cursorIndex + 1);
                            }

                            const beforeBang = textarea.value.substring(0, bangIndex);
                            const afterBang = textarea.value.substring(cursorPosition);
                            setInputText(beforeBang + "!" + complete + " " + afterBang);

                            // 设置光标位置
                            setTimeout(() => {
                                const newPosition =
                                    bangIndex + (cursorIndex === -1 ? selectedBang[0].length + 2 : cursorIndex + 1);
                                textarea.setSelectionRange(newPosition, newPosition);
                            }, 0);
                        }
                        setBangListVisible(false);
                    } else {
                        // Enter for submit (only if not in IME composition)
                        e.preventDefault();
                        handleSend();
                    }
                } else if (e.key === "Tab" && artifactListVisible) {
                    // Select artifact
                    e.preventDefault();
                    const selectedArtifact = filteredArtifacts[selectedArtifactIndex];
                    const textarea = e.currentTarget as HTMLTextAreaElement;
                    const cursorPosition = textarea.selectionStart;
                    const hashIndex = textarea.value.lastIndexOf("#", cursorPosition - 1);

                    if (hashIndex !== -1) {
                        const beforeHash = textarea.value.substring(0, hashIndex);
                        const afterHash = textarea.value.substring(cursorPosition);
                        setInputText(beforeHash + `#${selectedArtifact.name} ` + afterHash);

                        // 设置光标位置
                        setTimeout(() => {
                            const newPosition = hashIndex + selectedArtifact.name.length + 2;
                            textarea.setSelectionRange(newPosition, newPosition);
                        }, 0);
                    }
                    setArtifactListVisible(false);
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
                        textarea.value.lastIndexOf("！", cursorPosition - 1)
                    );

                    if (bangIndex !== -1) {
                        // 找到complete中的|的位置
                        const cursorIndex = complete.indexOf("|");
                        // 如果有|，则将光标移动到|的位置，并且移除|
                        if (cursorIndex !== -1) {
                            complete = complete.substring(0, cursorIndex) + complete.substring(cursorIndex + 1);
                        }

                        const beforeBang = textarea.value.substring(0, bangIndex);
                        const afterBang = textarea.value.substring(cursorPosition);
                        setInputText(beforeBang + "!" + complete + " " + afterBang);

                        // 设置光标位置
                        setTimeout(() => {
                            const newPosition =
                                bangIndex + (cursorIndex === -1 ? selectedBang[0].length + 2 : cursorIndex + 1);
                            textarea.setSelectionRange(newPosition, newPosition);
                        }, 0);
                    }
                    setBangListVisible(false);
                } else if (e.key === "ArrowUp" && artifactListVisible) {
                    e.preventDefault();
                    setSelectedArtifactIndex((prevIndex) =>
                        prevIndex > 0 ? prevIndex - 1 : filteredArtifacts.length - 1
                    );
                } else if (e.key === "ArrowDown" && artifactListVisible) {
                    e.preventDefault();
                    setSelectedArtifactIndex((prevIndex) =>
                        prevIndex < filteredArtifacts.length - 1 ? prevIndex + 1 : 0
                    );
                } else if (e.key === "ArrowUp" && assistantListVisible) {
                    e.preventDefault();
                    setSelectedAssistantIndex((prevIndex) =>
                        prevIndex > 0 ? prevIndex - 1 : filteredAssistants.length - 1
                    );
                } else if (e.key === "ArrowDown" && assistantListVisible) {
                    e.preventDefault();
                    setSelectedAssistantIndex((prevIndex) =>
                        prevIndex < filteredAssistants.length - 1 ? prevIndex + 1 : 0
                    );
                } else if (e.key === "ArrowUp" && bangListVisible) {
                    e.preventDefault();
                    setSelectedBangIndex((prevIndex) => (prevIndex > 0 ? prevIndex - 1 : bangList.length - 1));
                } else if (e.key === "ArrowDown" && bangListVisible) {
                    e.preventDefault();
                    setSelectedBangIndex((prevIndex) => (prevIndex < bangList.length - 1 ? prevIndex + 1 : 0));
                } else if (e.key === "Escape") {
                    e.preventDefault();
                    if (bangListVisible || assistantListVisible || artifactListVisible) {
                        // Hide completion lists
                        setBangListVisible(false);
                        setAssistantListVisible(false);
                        setArtifactListVisible(false);
                    } else {
                        // No completion lists visible, blur the textarea to remove focus
                        textareaRef.current?.blur();
                    }
                }
            };

            function scrollToSelectedBang() {
                const selectedBangElement = document.querySelector(".completion-bang-container.selected");
                if (selectedBangElement) {
                    const parentElement = selectedBangElement.parentElement;
                    if (parentElement) {
                        const parentRect = parentElement.getBoundingClientRect();
                        const selectedRect = selectedBangElement.getBoundingClientRect();

                        if (selectedRect.top < parentRect.top) {
                            parentElement.scrollTop -= parentRect.top - selectedRect.top;
                        } else if (selectedRect.bottom > parentRect.bottom) {
                            parentElement.scrollTop += selectedRect.bottom - parentRect.bottom;
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
                            onFocus={() => {
                                setIsFocused(true);
                                adjustTextareaHeight();
                            }}
                            onBlur={() => {
                                setIsFocused(false);
                                restoreInitialHeight();
                            }}
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

                    <ArtifactCompletionList
                        artifactListVisible={artifactListVisible}
                        placement={placement}
                        cursorPosition={cursorPosition}
                        artifacts={filteredArtifacts}
                        selectedArtifactIndex={selectedArtifactIndex}
                        textareaRef={textareaRef}
                        setInputText={setInputText}
                        setArtifactListVisible={setArtifactListVisible}
                        onArtifactSelect={(artifact, action) => {
                            if (action === "open") {
                                // Open artifact when Enter is pressed
                                invoke("open_artifact_window", { artifactId: artifact.id }).catch((error) => {
                                    console.error("Failed to open artifact:", error);
                                });
                            } else {
                                // Tab key completion - just log for now
                                console.log("Artifact completed:", artifact.name);
                            }
                        }}
                    />
                </div>
            );
        }
    )
);

export default InputArea;
