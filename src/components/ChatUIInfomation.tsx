import IconButton from "./IconButton";
import Setting from "../assets/setting.svg?react";
import Experiment from "../assets/experiment.svg?react";
import { invoke } from "@tauri-apps/api/core";
import AnimatedLogo from "./AnimatedLogo";
import { useLogoState } from "../hooks/useLogoState";

function ChatUIInfomation() {
    const {
        state: logoState,
        showHappy,
        showError,
        showNormal,
    } = useLogoState({
        defaultState: "happy",
        autoReturnToNormal: true,
        autoReturnDelay: 3000,
    });

    const openConfig = async () => {
        try {
            await invoke("open_config_window");
            showHappy();
        } catch (error) {
            showError();
        }
    };

    const openPlugin = async () => {
        try {
            await invoke("open_plugin_window");
            showHappy();
        } catch (error) {
            showError();
        }
    };

    return (
        <div className="flex justify-between py-4 px-5 border-gray-200">
            <div className="flex items-center gap-2">
                <AnimatedLogo
                    state={logoState}
                    size={32}
                    onClick={showNormal}
                />
            </div>
            <div className="flex items-center gap-2">
                <IconButton
                    icon={<Setting fill="black" />}
                    onClick={openConfig}
                />
                <IconButton
                    icon={<Experiment fill="black" />}
                    onClick={openPlugin}
                />
            </div>
        </div>
    );
}

export default ChatUIInfomation;
