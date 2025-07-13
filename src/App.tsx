import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import './App.css';
import AskWindow from "./AskWindow.tsx";
import ConfigWindow from "./ConfigWindow.tsx";
import ChatUIWindow from './ChatUIWindow.tsx';
import PreviewHTMLWindow from './artifacts/PreviewHTMLWindow.tsx';
import ArtifactPreviewWindow from './artifacts/ArtifactPreviewWindow.tsx';
import PluginWindow from './PluginWindow.tsx';
import { Toaster } from './components/ui/sonner.tsx';

const windowsMap: Record<string, typeof AskWindow> = {
    ask: AskWindow,
    config: ConfigWindow,
    chat_ui: ChatUIWindow,
    preview_html: PreviewHTMLWindow,
    artifact_preview: ArtifactPreviewWindow,
    plugin: PluginWindow
}

function App() {
    let win = getCurrentWebviewWindow();

    return <>
        {windowsMap[win.label]()}
        <Toaster richColors />
    </>
}

export default App;
