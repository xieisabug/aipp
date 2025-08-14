import React, { ReactNode, useEffect, useState } from "react";
import LLMProviderConfig from "./components/config/LLMProviderConfig";
import AssistantConfig from "./components/config/AssistantConfig";
import FeatureAssistantConfig from "./components/config/FeatureAssistantConfig";
import MCPConfig from "./components/config/MCPConfig";
import { appDataDir } from "@tauri-apps/api/path";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Blocks, Bot, ServerCrash, Settings } from "lucide-react";

interface MenuItem {
    id: string;
    name: string;
    icon: ReactNode;
    iconSelected: ReactNode;
}

// 将 contentMap 修改为映射到组件而不是元素
const contentMap: Record<string, React.ComponentType<any>> = {
    "llm-provider-config": LLMProviderConfig,
    "assistant-config": AssistantConfig,
    "feature-assistant-config": FeatureAssistantConfig,
    "mcp-config": MCPConfig,
};

function ConfigWindow() {
    const menuList: Array<MenuItem> = [
        {
            id: "llm-provider-config",
            name: "大模型配置",
            icon: <ServerCrash color="#6b7280" className="w-full h-full" />,
            iconSelected: (
                <ServerCrash color="#111827" className="w-full h-full" />
            ),
        },
        {
            id: "assistant-config",
            name: "个人助手配置",
            icon: <Bot color="#6b7280" className="w-full h-full" />,
            iconSelected: <Bot color="#111827" className="w-full h-full" />,
        },
        {
            id: "mcp-config",
            name: "MCP管理",
            icon: <Blocks color="#6b7280" className="w-full h-full" />,
            iconSelected: <Blocks color="#111827" className="w-full h-full" />,
        },
        {
            id: "feature-assistant-config",
            name: "程序配置",
            icon: <Settings color="#6b7280" className="w-full h-full" />,
            iconSelected: (
                <Settings color="#111827" className="w-full h-full" />
            ),
        },
    ];

    const [selectedMenu, setSelectedMenu] = useState<string>(
        "llm-provider-config",
    );
    const [pluginList, setPluginList] = useState<any[]>([]);

    useEffect(() => {
        const pluginLoadList = [
            {
                name: "代码生成",
                code: "code-generate",
                pluginType: ["assistantType"],
                instance: null,
            },
        ];

        const initPlugin = async () => {
            const dirPath = await appDataDir();
            pluginLoadList.forEach(async (plugin) => {
                const convertFilePath =
                    dirPath + "/plugin/" + plugin.code + "/dist/main.js";

                // 加载脚本
                const script = document.createElement("script");
                script.src = convertFileSrc(convertFilePath);
                script.onload = () => {
                    // 脚本加载完成后，插件应该可以在全局范围内使用
                    const SamplePlugin = (window as any).SamplePlugin;
                    if (SamplePlugin) {
                        const instance = new SamplePlugin();
                        plugin.instance = instance;
                        console.log("plugin loaded", instance);
                    }
                };
                document.body.appendChild(script);
            });

            setPluginList(pluginLoadList);
        };

        initPlugin();
    }, []);

    // 获取选中的组件
    const SelectedComponent = contentMap[selectedMenu];

    // 导航函数
    const navigateTo = (menuKey: string) => {
        if (contentMap[menuKey]) {
            setSelectedMenu(menuKey);
        }
    };

    return (
        <div className="flex justify-center items-center h-screen bg-background">
            <div
                className="bg-card shadow-lg w-full h-screen grid grid-cols-[1fr_3fr] md:grid-cols-[1fr_4fr] lg:grid-cols-[1fr_5fr]"
                data-tauri-drag-region
            >
                {/* 侧边栏 */}
                <div className="bg-muted/30 border-r border-border px-3 md:px-4 py-6 overflow-y-auto">
                    <div className="flex flex-col gap-1 mt-2">
                        {menuList.map((item, index) => (
                            <div
                                key={index}
                                className={`
                                    relative flex items-center px-3 md:px-4 lg:px-5 py-3 md:py-3.5 rounded-lg cursor-pointer
                                    transition-all duration-200 ease-out font-medium text-xs md:text-sm
                                    select-none hover:translate-x-0.5
                                    ${
                                        selectedMenu === item.id
                                            ? "bg-gray-100 text-gray-900 font-semibold shadow-sm"
                                            : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
                                    }
                                `}
                                onClick={() => setSelectedMenu(item.id)}
                            >
                                {/* 选中状态的左侧指示条 */}
                                {selectedMenu === item.id && (
                                    <div className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-5 bg-gray-800 rounded-r-sm" />
                                )}
                                <div className="flex items-center">
                                    <div className="w-4 h-4 md:w-5 md:h-5 flex-shrink-0 mr-2 md:mr-3 lg:mr-3.5">
                                        {selectedMenu === item.id
                                            ? item.iconSelected
                                            : item.icon}
                                    </div>
                                    <span className="truncate">
                                        {item.name}
                                    </span>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>

                {/* 内容区域 */}
                <div className="bg-card px-4 md:px-6 lg:px-8 py-6 overflow-y-auto max-h-screen">
                    {/* 配置组件内容 */}
                    <SelectedComponent
                        pluginList={pluginList}
                        navigateTo={navigateTo}
                    />
                </div>
            </div>
        </div>
    );
}

export default ConfigWindow;
