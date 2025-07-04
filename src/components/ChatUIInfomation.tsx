import IconButton from "./IconButton";
import Setting from "../assets/setting.svg?react";
import Experiment from "../assets/experiment.svg?react";
import { invoke } from "@tauri-apps/api/core";

function ChatUIInfomation() {
    const openConfig = async () => {
        await invoke('open_config_window')
    }

    const openPlugin = async () => {
        await invoke('open_plugin_window')
    }

    return (
        <div className="flex justify-between py-4 px-5 border-b border-gray-200 bg-white rounded-t-xl">
            <h1 className="text-primary text-3xl">Aipp</h1>
            <div className="flex items-center gap-2">
                <IconButton icon={<Setting fill="black" />} onClick={openConfig} border />
                <IconButton icon={<Experiment fill="black" />} onClick={openPlugin} border />
            </div>
        </div>
    );
}

export default ChatUIInfomation;
