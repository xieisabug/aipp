import React, { useEffect, useState } from 'react';
import ReactDOM from 'react-dom';
import { appDataDir } from '@tauri-apps/api/path';
import { convertFileSrc } from '@tauri-apps/api/core';

function PluginWindow() {
    window.React = React;
    window.ReactDOM = ReactDOM;
    const [pluginNode, setPluginNode] = useState<React.ReactNode>(null);

    useEffect(() => {
        const initPage = async () => {
            const dirPath = await appDataDir();
            const convertFilePath = dirPath + "plugin/tea-plugin-template/dist/main.js";

            // 加载脚本
            const script = document.createElement('script');
            script.src = convertFileSrc(convertFilePath);
            script.onload = () => {
                // 脚本加载完成后，插件应该可以在全局范围内使用
                const SamplePlugin = (window as any).SamplePlugin;
                if (SamplePlugin) {
                    const instance = new SamplePlugin();
                    if (typeof instance.onPluginLoad === 'function') {
                        instance.onPluginLoad();
                        setPluginNode(instance.renderComponent());
                    }
                }
            };
            document.body.appendChild(script);

        }

        initPage();
    }, []);

    return (
        <div style={{ backgroundColor: "white", width: "100vw", height: "100vh" }} data-tauri-drag-region>
            {
                pluginNode === null ?
                    <h1>正在加载插件...</h1> :
                    pluginNode
            }
        </div>
    );
};

export default PluginWindow;