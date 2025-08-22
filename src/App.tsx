import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./App.css";
import AskWindow from "./windows/AskWindow.tsx";
import ConfigWindow from "./windows/ConfigWindow.tsx";
import ChatUIWindow from "./windows/ChatUIWindow.tsx";
import ArtifactPreviewWindow from "./windows/ArtifactPreviewWindow.tsx";
import PluginWindow from "./windows/PluginWindow.tsx";
import ArtifactCollectionsWindow from "./windows/ArtifactCollectionsWindow.tsx";
import ArtifactWindow from "./windows/ArtifactWindow.tsx";
import PluginStoreWindow from "./windows/PluginStoreWindow.tsx";
import CodeThemeLoader from "./components/CodeThemeLoader.tsx";
import { Toaster } from "./components/ui/sonner.tsx";

const windowsMap: Record<string, typeof AskWindow> = {
    ask: AskWindow,
    config: ConfigWindow,
    chat_ui: ChatUIWindow,
    artifact_preview: ArtifactPreviewWindow,
    plugin: PluginWindow,
    plugin_store: PluginStoreWindow,
    artifact_collections: ArtifactCollectionsWindow,
    artifact: ArtifactWindow,
};

function getWindowComponent(label: string) {
    return windowsMap[label];
}

function App() {
    let win = getCurrentWebviewWindow();
    const WindowComponent = getWindowComponent(win.label);

    return (
        <CodeThemeLoader>
            {WindowComponent ? WindowComponent() : <div>未知窗口类型: {win.label}</div>}
            <Toaster richColors />
        </CodeThemeLoader>
    );
}

export default App;
