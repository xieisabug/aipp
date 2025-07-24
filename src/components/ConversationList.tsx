import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import MenuIcon from "../assets/menu.svg?react";
import FormDialog from "./FormDialog";
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
    const { deleteConversation, listConversations } = useConversationManager();

    useEffect(() => {
        listConversations().then((c) => {
            setConversations(c);
        });
    }, []);

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
            listConversations().then((c) => {
                setConversations(c);
                // 获取完毕后重置标记，防止死循环
                fetchRetryingRef.current = false;
            });
        }
    }, [conversationId, conversations]);

    useEffect(() => {
        const unsubscribe = listen("title_change", (event) => {
            const [conversationId, title] = event.payload as [string, string];

            const index = conversations.findIndex(
                (conversation) => conversation.id.toString() == conversationId,
            );
            if (index !== -1) {
                const newConversations = [...conversations];
                newConversations[index] = {
                    ...newConversations[index],
                    name: title,
                };
                setConversations(newConversations);
            }
        });

        const index = conversations.findIndex(
            (c) => conversationId == c.id.toString(),
        );
        if (index === -1) {
            onSelectConversation("");
        }

        return () => {
            if (unsubscribe) {
                unsubscribe.then((f) => f());
            }
        };
    }, [conversations]);

    const [menuShow, setMenuShow] = useState(false);
    const [menuShowConversationId, setMenuShowConversationId] = useState("");

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

    const [formDialogIsOpen, setFormDialogIsOpen] = useState<boolean>(false);
    const openFormDialog = useCallback((title: string) => {
        setFormConversationTitle(title || "");
        setFormDialogIsOpen(true);
    }, []);
    const closeFormDialog = useCallback(() => {
        setFormDialogIsOpen(false);
    }, []);
    const [formConversationTitle, setFormConversationTitle] =
        useState<string>("");

    const [deleteDialogIsOpen, setDeleteDialogIsOpen] = useState<boolean>(false);
    const [deleteConversationId, setDeleteConversationId] = useState<string>("");
    const [deleteConversationName, setDeleteConversationName] = useState<string>("");
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

    const handleFormSubmit = useCallback(() => {
        if (
            menuShowConversationId === "" ||
            menuShowConversationId === undefined
        ) {
            // TODO 弹出错误提示
            console.error("menuShowConversationId is empty");
        }
        invoke("update_conversation", {
            conversationId: +menuShowConversationId,
            name: formConversationTitle,
        }).then(() => {
            const newConversations = conversations.map((conversation) => {
                if (conversation.id.toString() === menuShowConversationId) {
                    return { ...conversation, name: formConversationTitle };
                }
                return conversation;
            });
            setConversations(newConversations);
            closeFormDialog();
        });
    }, [menuShowConversationId, formConversationTitle]);

    return (
        <div className="flex-1 overflow-y-auto overflow-x-hidden px-3">
            <ul className="list-none p-0 m-0">
                {conversations.map((conversation) => (
                    <li
                        className={`group h-16 w-full mx-0 mb-2 text-sm border-0 rounded-xl cursor-pointer flex flex-col justify-center p-3 box-border relative transition-all duration-200 ${conversationId == conversation.id.toString() ? "font-bold text-primary bg-primary-foreground" : "bg-transparent hover:bg-slate-50 hover:translate-x-0.5"}`}
                        key={conversation.id}
                        onClick={() => {
                            onSelectConversation(conversation.id.toString());
                        }}
                    >
                        <div className="overflow-hidden text-ellipsis whitespace-nowrap font-medium">
                            {conversation.name}
                        </div>
                        <div className="text-xs overflow-hidden text-ellipsis whitespace-nowrap text-gray-500">
                            {conversation.assistant_name}
                        </div>

                        <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                                <Button
                                    variant="link"
                                    className="invisible absolute right-2 top-4 group-hover:visible transition-opacity duration-200"
                                >
                                    <MenuIcon fill={"black"} />
                                </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent>
                                <DropdownMenuItem
                                    onClick={() => {
                                        setMenuShow(false);
                                        setMenuShowConversationId(
                                            conversationId,
                                        );
                                        openFormDialog(conversation.name);
                                    }}
                                >
                                    修改标题
                                </DropdownMenuItem>
                                <DropdownMenuItem
                                    onClick={() => {
                                        setMenuShow(false);
                                        openDeleteDialog(
                                            conversation.id.toString(),
                                            conversation.name
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

            <FormDialog
                title={"修改对话标题"}
                onSubmit={handleFormSubmit}
                onClose={closeFormDialog}
                isOpen={formDialogIsOpen}
            >
                <form className="space-y-4">
                    <div className="space-y-2">
                        <label className="block text-sm font-medium text-gray-700">标题:</label>
                        <input
                            className="w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                            type="text"
                            name="name"
                            value={formConversationTitle}
                            onChange={(e) =>
                                setFormConversationTitle(e.target.value)
                            }
                        />
                    </div>
                </form>
            </FormDialog>

            <ConfirmDialog
                title={"确认删除对话"}
                confirmText={`确定要删除对话 "${deleteConversationName}" 吗？此操作无法撤销。`}
                onConfirm={() => {
                    deleteConversation(deleteConversationId, {
                        onSuccess: async () => {
                            const conversations = await listConversations();
                            setConversations(conversations);
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
