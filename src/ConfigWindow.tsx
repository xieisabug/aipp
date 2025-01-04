import React, { ReactNode, useEffect, useState } from "react";
import SideMenu from "./components/SideMenu";
import LLMProviderConfig from "./components/config/LLMProviderConfig";
import AssistantConfig from "./components/config/AssistantConfig";
import FeatureAssistantConfig from "./components/config/FeatureAssistantConfig";
import Model from "./assets/model.svg?react";
import Assistant from "./assets/assistant.svg?react";
import Program from "./assets/program.svg?react";
import { appDataDir } from "@tauri-apps/api/path";
import { convertFileSrc } from "@tauri-apps/api/core";

interface MenuItem {
    id: string;
    name: string;
    icon: ReactNode;
    iconSelected: ReactNode;
}

// 将 contentMap 修改为映射到组件而不是元素
const contentMap: Record<string, React.ComponentType<any>> = {
    'llm-provider-config': LLMProviderConfig,
    'assistant-config': AssistantConfig,
    'feature-assistant-config': FeatureAssistantConfig,
}

function ConfigWindow() {
    const menuList: Array<MenuItem> = [
        { id: 'llm-provider-config', name: '大模型配置', icon: <Model fill="gray" />, iconSelected: <Model fill="black" /> },
        { id: 'assistant-config', name: '个人助手配置', icon: <Assistant fill="gray" />, iconSelected: <Assistant fill="black" /> },
        { id: 'feature-assistant-config', name: '程序助手配置', icon: <Program fill="gray" />, iconSelected: <Program fill="black" /> },
    ];

    const [selectedMenu, setSelectedMenu] = useState<string>('llm-provider-config');
    const [pluginList, setPluginList] = useState<any[]>([]);

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
            pluginLoadList.forEach(async (plugin) => {
                const convertFilePath = dirPath + "/plugin/" + plugin.code + "/dist/main.js";

                // 加载脚本
                const script = document.createElement('script');
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

            setPluginList(pluginLoadList)
        }

        initPlugin();
    }, []);

    // 获取选中的组件
    const SelectedComponent = contentMap[selectedMenu];

    return (
        <div className="mx-auto grid md:grid-cols-[210px_1fr] lg:grid-cols-[250px_1fr] bg-background">
            <SideMenu menu={menuList} selectedMenu={selectedMenu} setSelectedMenu={setSelectedMenu} />
            <div className="max-h-screen overflow-auto">
                <SelectedComponent pluginList={pluginList} />
            </div>
        </div>
    );
}

export default ConfigWindow;
