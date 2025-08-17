import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import MenuIcon from "../assets/menu.svg?react";
import ConversationTitleEditDialog from "./ConversationTitleEditDialog";
import useConversationManager from "../hooks/useConversationManager";
import { Conversation } from "../data/Conversation";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Button } from "./ui/button";
import ConfirmDialog from "./ConfirmDialog";

interface ConversationListProps {
    onSelectConversation: (conversation: string) => void;
    conversationId: string;
}

function ConversationList({
    onSelectConversation,
    conversationId,
}: ConversationListProps) {
    const [conversations, setConversations] = useState<Array<Conversation>>([]);
    const [currentPage, setCurrentPage] = useState(1);
    const [isLoading, setIsLoading] = useState(false);
    const [hasMoreData, setHasMoreData] = useState(true);
    const [isLoadingMore, setIsLoadingMore] = useState(false);
    const scrollContainerRef = useRef<HTMLDivElement>(null);
    const { deleteConversation, listConversations } = useConversationManager();

    useEffect(() => {
        setIsLoading(true);
        listConversations(1, 100)
            .then((c) => {
                setConversations(c);
                setHasMoreData(c.length === 100); // 如果返回的数据等于页面大小，可能还有更多数据
                setIsLoading(false);
            })
            .catch(() => {
                setIsLoading(false);
            });
    }, []);

    // 加载下一页数据
    const loadNextPage = useCallback(async () => {
        if (isLoadingMore || !hasMoreData) return;

        setIsLoadingMore(true);
        try {
            const nextPage = currentPage + 1;
            const newConversations = await listConversations(nextPage, 100);

            if (newConversations.length > 0) {
                setConversations((prev) => [...prev, ...newConversations]);
                setCurrentPage(nextPage);
                setHasMoreData(newConversations.length === 100);
            } else {
                setHasMoreData(false);
            }
        } catch (error) {
            console.error("Failed to load more conversations:", error);
        } finally {
            setIsLoadingMore(false);
        }
    }, [currentPage, hasMoreData, isLoadingMore, listConversations]);

    // 滚动监听，检测是否滚动到最后10%
    useEffect(() => {
        const container = scrollContainerRef.current;
        if (!container) return;

        const handleScroll = () => {
            const { scrollTop, scrollHeight, clientHeight } = container;
            const scrollPercentage = (scrollTop + clientHeight) / scrollHeight;

            // 当滚动到最后10%时加载下一页
            if (scrollPercentage >= 0.9 && hasMoreData && !isLoadingMore) {
                loadNextPage();
            }
        };

        container.addEventListener("scroll", handleScroll);
        return () => container.removeEventListener("scroll", handleScroll);
    }, [hasMoreData, isLoadingMore, loadNextPage]);

    // 当 conversationId 不在当前列表中时，自动重新拉取列表。
    const fetchRetryingRef = useRef(false);
    useEffect(() => {
        if (!conversationId) {
            return;
        }

        const exists = conversations.some(
            (conversation) => conversation.id.toString() === conversationId,
        );

        if (!exists && !fetchRetryingRef.current) {
            console.log(
                "ConversationList: 选中的 conversationId 不在列表中，重新获取列表...",
                conversationId,
            );
            fetchRetryingRef.current = true;
            listConversations(1, 100).then((c) => {
                setConversations(c);
                setCurrentPage(1);
                setHasMoreData(c.length === 100);
                // 获取完毕后重置标记，防止死循环
                fetchRetryingRef.current = false;
            });
        }
    }, [conversationId, conversations]);

    useEffect(() => {
        const index = conversations.findIndex(
            (c) => conversationId == c.id.toString(),
        );
        if (index === -1) {
            onSelectConversation("");
        }
    }, [conversations]);

    const [menuShow, setMenuShow] = useState(false);

    useEffect(() => {
        const handleOutsideClick = () => {
            if (menuShow) {
                setMenuShow(false);
            }
        };

        document.addEventListener("click", handleOutsideClick);

        return () => {
            document.removeEventListener("click", handleOutsideClick);
        };
    }, [menuShow]);

    const [titleEditDialogIsOpen, setTitleEditDialogIsOpen] =
        useState<boolean>(false);
    const [editingConversationId, setEditingConversationId] =
        useState<number>(0);
    const [editingConversationTitle, setEditingConversationTitle] =
        useState<string>("");

    const openTitleEditDialog = useCallback((id: number, title: string) => {
        setEditingConversationId(id);
        setEditingConversationTitle(title || "");
        setTitleEditDialogIsOpen(true);
    }, []);

    const closeTitleEditDialog = useCallback(() => {
        setTitleEditDialogIsOpen(false);
        setEditingConversationId(0);
        setEditingConversationTitle("");
    }, []);

    // 监听标题变化事件
    useEffect(() => {
        const unsubscribe = listen("title_change", (event) => {
            const [conversationIdFromEvent, title] = event.payload as [
                number,
                string,
            ];

            const index = conversations.findIndex(
                (conversation) => conversation.id === conversationIdFromEvent,
            );
            if (index !== -1) {
                const newConversations = [...conversations];
                newConversations[index] = {
                    ...newConversations[index],
                    name: title,
                };
                setConversations(newConversations);

                // 如果当前正在编辑这个对话的标题，也更新编辑状态
                if (
                    editingConversationId === conversationIdFromEvent &&
                    titleEditDialogIsOpen
                ) {
                    setEditingConversationTitle(title);
                }
            }
        });

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, [conversations, editingConversationId, titleEditDialogIsOpen]);

    // 监听对话删除事件
    useEffect(() => {
        const unsubscribe = listen("conversation_deleted", (event) => {
            const deletedConversationId = event.payload as number;

            // 从列表中移除被删除的对话
            setConversations((prevConversations) =>
                prevConversations.filter(
                    (conversation) => conversation.id !== deletedConversationId
                )
            );
        });

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, []);

    const [deleteDialogIsOpen, setDeleteDialogIsOpen] =
        useState<boolean>(false);
    const [deleteConversationId, setDeleteConversationId] =
        useState<string>("");
    const [deleteConversationName, setDeleteConversationName] =
        useState<string>("");
    const openDeleteDialog = useCallback((id: string, name: string) => {
        setDeleteConversationId(id);
        setDeleteConversationName(name);
        setDeleteDialogIsOpen(true);
    }, []);
    const closeDeleteDialog = useCallback(() => {
        setDeleteDialogIsOpen(false);
        setDeleteConversationId("");
        setDeleteConversationName("");
    }, []);

    return (
        <div
            className="flex-1 overflow-y-auto overflow-x-hidden px-3 bg-background"
            ref={scrollContainerRef}
        >
            <ul className="list-none p-0 m-0">
                {conversations.map((conversation) => (
                    <li
                        className={`group h-16 w-full mx-0 mb-2 text-sm border-0 rounded-xl cursor-pointer select-none flex flex-col justify-center p-3 box-border relative transition-all duration-200 ${conversationId == conversation.id.toString() ? "font-bold text-primary bg-primary-foreground" : "bg-transparent hover:bg-muted hover:translate-x-0.5"}`}
                        key={conversation.id}
                        onClick={() => {
                            onSelectConversation(conversation.id.toString());
                        }}
                    >
                        <div className="overflow-hidden text-ellipsis whitespace-nowrap font-medium">
                            {conversation.name}
                        </div>
                        <div className="text-xs overflow-hidden text-ellipsis whitespace-nowrap text-muted-foreground">
                            {conversation.assistant_name}
                        </div>

                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <Button
                                    variant="link"
                                    className="invisible absolute right-2 top-4 group-hover:visible transition-opacity duration-200"
                                >
                                    <MenuIcon className="fill-foreground" />
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent>
                                <DropdownMenuItem
                                    onClick={() => {
                                        setMenuShow(false);
                                        openTitleEditDialog(
                                            conversation.id,
                                            conversation.name,
                                        );
                                    }}
                                >
                                    修改标题
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => {
                                        setMenuShow(false);
                                        openDeleteDialog(
                                            conversation.id.toString(),
                                            conversation.name,
                                        );
                                    }}
                                >
                                    删除
                                </DropdownMenuItem>
                            </DropdownMenuContent>
                        </DropdownMenu>
                    </li>
                ))}
            </ul>

            {/* 加载指示器 */}
            {isLoading && conversations.length === 0 && (
                <div className="flex justify-center items-center py-8">
                    <div className="text-sm text-muted-foreground">加载中...</div>
                </div>
            )}

            {isLoadingMore && (
                <div className="flex justify-center items-center py-4">
                    <div className="text-sm text-muted-foreground">加载更多...</div>
                </div>
            )}

            {!hasMoreData && conversations.length > 0 && (
                <div className="flex justify-center items-center py-4">
                    <div className="text-xs text-muted-foreground">已加载全部对话</div>
                </div>
            )}

            <ConversationTitleEditDialog
                isOpen={titleEditDialogIsOpen}
                conversationId={editingConversationId}
                initialTitle={editingConversationTitle}
                onClose={closeTitleEditDialog}
            />

            <ConfirmDialog
                title={"确认删除对话"}
                confirmText={`确定要删除对话 "${deleteConversationName}" 吗？此操作无法撤销。`}
                onConfirm={() => {
                    deleteConversation(deleteConversationId, {
                        onSuccess: async () => {
                            // 删除后重新加载第一页数据
                            setIsLoading(true);
                            try {
                                const conversations = await listConversations(
                                    1,
                                    100,
                                );
                                setConversations(conversations);
                                setCurrentPage(1);
                                setHasMoreData(conversations.length === 100);
                            } catch (error) {
                                console.error(
                                    "Failed to reload conversations after delete:",
                                    error,
                                );
                            } finally {
                                setIsLoading(false);
                            }
                            closeDeleteDialog();
                        },
                    });
                }}
                onCancel={closeDeleteDialog}
                isOpen={deleteDialogIsOpen}
            />
        </div>
    );
}

export default ConversationList;
