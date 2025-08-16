import { useEffect, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import ChatUIToolbar from "./components/ChatUIToolbar";
import ConversationList from "./components/ConversationList";
import ChatUIInfomation from "./components/ChatUIInfomation";
import ConversationUI from "./components/ConversationUI";

import { appDataDir } from "@tauri-apps/api/path";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

function ChatUIWindow() {
    const [pluginList, setPluginList] = useState<any[]>([]);

    const [selectedConversation, setSelectedConversation] = useState<string>("");

    // 组件挂载完成后，发送窗口加载事件，通知 AskWindow
    useEffect(() => {
        emit("chat-ui-window-load");

        // 监听 AskWindow 发来的选中对话事件 (只注册一次)
        const unlisten = getCurrentWebviewWindow().listen("select_conversation", (event) => {
            const convId = event.payload;
            if (convId && convId !== "") {
                setSelectedConversation(convId as string);
            }
        });

        return () => {
            unlisten.then((unlisten) => unlisten());
        };
    }, []);

    useEffect(() => {
        const pluginLoadList = [
            {
                name: "代码生成",
                code: "code-generate",
                pluginType: ["assistantType"],
                instance: null
            }
        ]

        const initPlugin = async () => {
            const dirPath = await appDataDir();
            const loadPromises = pluginLoadList.map(async (plugin) => {
                const convertFilePath = dirPath + "plugin/" + plugin.code + "/dist/main.js";

                return new Promise<void>((resolve) => {
                    const script = document.createElement('script');
                    script.src = convertFileSrc(convertFilePath);
                    script.onload = () => {
                        const SamplePlugin = (window as any).SamplePlugin;
                        if (SamplePlugin) {
                            plugin.instance = new SamplePlugin();
                            console.log("plugin loaded", plugin.instance);
                        }
                        resolve();
                    };
                    document.body.appendChild(script);
                });
            });

            // 等待所有插件加载完成
            await Promise.all(loadPromises);

            // 所有插件实例都准备好后再更新状态
            setPluginList([...pluginLoadList]);
            console.log("setPluginList");
        }

        initPlugin();
    }, []);

    return (
        <div className="flex h-screen bg-gray-100">
            <div className="flex-none w-[280px] flex flex-col shadow-lg box-border rounded-r-xl my-2 mr-2">
                <ChatUIInfomation />
                <ChatUIToolbar onNewConversation={() => setSelectedConversation("")} />
                <ConversationList conversationId={selectedConversation} onSelectConversation={setSelectedConversation} />
            </div>

            <div className="flex-1 bg-white overflow-auto rounded-xl m-2 ml-0 shadow-lg">
                <ConversationUI pluginList={pluginList} conversationId={selectedConversation} onChangeConversationId={setSelectedConversation} />
            </div>
        </div>
    );
}

export default ChatUIWindow;