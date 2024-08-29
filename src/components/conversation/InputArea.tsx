import React from "react";
import CircleButton from "../CircleButton";
import Add from "../../assets/add.svg?react";
import Stop from "../../assets/stop.svg?react";
import UpArrow from "../../assets/up-arrow.svg?react";
import Delete from "../../assets/delete.svg?react";
import { AttachmentType, FileInfo } from "../../data/Conversation";
import IconButton from "../IconButton";

const InputArea: React.FC<{
    inputText: string;
    setInputText: (text: string) => void;
    handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
    fileInfoList: FileInfo[] | null;
    handleChooseFile: () => void;
    handlePaste: (e: React.ClipboardEvent<HTMLTextAreaElement>) => void;
    handleDeleteFile: (fileId: number) => void;
    handleSend: () => void;
    aiIsResponsing: boolean;
}> = React.memo(({ inputText, setInputText, handleKeyDown, fileInfoList, handleChooseFile, handlePaste, handleDeleteFile, handleSend, aiIsResponsing }) => (
    <div className="input-area">
        <div className="input-area-img-container">
            {fileInfoList?.map((fileInfo) => (
                <div key={fileInfo.name + fileInfo.id} className={
                    fileInfo.type === AttachmentType.Image ? "input-area-img-wrapper" : "other-class-name" // 替换 "other-class-name" 为实际的 className
                }>
                    {(() => {
                        switch (fileInfo.type) {
                            case AttachmentType.Image:
                                return <img src={fileInfo.thumbnail} alt="缩略图" className="input-area-img" />;
                            case AttachmentType.Text:
                                return <span>{fileInfo.name}</span>;
                            case AttachmentType.PDF:
                                return <span>{fileInfo.name} (PDF)</span>;
                            case AttachmentType.Word:
                                return <span>{fileInfo.name} (Word)</span>;
                            case AttachmentType.PowerPoint:
                                return <span>{fileInfo.name} (PowerPoint)</span>;
                            case AttachmentType.Excel:
                                return <span>{fileInfo.name} (Excel)</span>;
                            default:
                                return <span>{fileInfo.name}</span>;
                        }
                    })()}
                    <IconButton border icon={<Delete fill="black" />} className="input-area-img-delete-button" onClick={() => {handleDeleteFile(fileInfo.id)}} />
                </div>
            ))}
        </div>
        <textarea
            className="input-area-textarea"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
        />
        <CircleButton onClick={handleChooseFile} icon={<Add fill="black" />} className="input-area-add-button" />
        <CircleButton size="large" onClick={handleSend} icon={aiIsResponsing ? <Stop width={20} height={20} fill="white" /> : <UpArrow width={20} height={20} fill="white" />} primary className="input-area-send-button" />
    </div>
));

export default InputArea;