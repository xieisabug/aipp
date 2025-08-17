import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import './App.css';
import AskWindow from "./AskWindow.tsx";
import ConfigWindow from "./ConfigWindow.tsx";
import ChatUIWindow from './ChatUIWindow.tsx';
import ArtifactPreviewWindow from './artifacts/ArtifactPreviewWindow.tsx';
import PluginWindow from './PluginWindow.tsx';
import ArtifactCollectionsWindow from './windows/ArtifactCollectionsWindow.tsx';
import ArtifactWindow from './windows/ArtifactWindow.tsx';
import { Toaster } from './components/ui/sonner.tsx';

const windowsMap: Record<string, typeof AskWindow> = {
    ask: AskWindow,
    config: ConfigWindow,
    chat_ui: ChatUIWindow,
    artifact_preview: ArtifactPreviewWindow,
    plugin: PluginWindow,
    artifact_collections: ArtifactCollectionsWindow,
}

// 处理动态 artifact 窗口（artifact_1, artifact_2 等）
function getWindowComponent(label: string) {
    if (label.startsWith('artifact_') && label !== 'artifact_preview' && label !== 'artifact_collections') {
        return ArtifactWindow;
    }
    return windowsMap[label];
}

function App() {
    let win = getCurrentWebviewWindow();
    const WindowComponent = getWindowComponent(win.label);

    return <>
        {WindowComponent ? WindowComponent() : <div>未知窗口类型: {win.label}</div>}
        <Toaster richColors />
    </>
}

export default App;
