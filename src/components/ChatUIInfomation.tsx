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

    const openArtifactsCollections = async () => {
        try {
            await invoke("open_artifact_collections_window");
            showHappy();
        } catch (error) {
            showError();
        }
    };

    return (
        <div className="flex justify-between py-4 px-5 border-border">
            <div className="flex items-center gap-2">
                <AnimatedLogo
                    state={logoState}
                    size={32}
                    onClick={showNormal}
                />
            </div>
            <div className="flex items-center gap-2">
                <IconButton
                    icon={<Setting className="fill-foreground" />}
                    onClick={openConfig}
                />
                <IconButton
                    icon={<Experiment className="fill-foreground" />}
                    onClick={openArtifactsCollections}
                />
            </div>
        </div>
    );
}

export default ChatUIInfomation;
