import React from 'react';

interface AlertDialogProps {
    alertText: string;
    alertType: 'success' | 'warning' | 'error';
    isOpen: boolean;
    onClose: () => void;
}

export interface AlertDialogParam {
    text: string;
    type: 'success' | 'warning' | 'error';
}

const AlertDialog: React.FC<AlertDialogProps> = ({ alertText, alertType, isOpen, onClose }) => {
    if (!isOpen) return null;

    const getAlertTitle = () => {
        switch (alertType) {
            case 'success':
                return '成功';
            case 'warning':
                return '警告';
            case 'error':
                return '错误';
            default:
                return '提示';
        }
    };

    const getBorderColorClass = () => {
        switch (alertType) {
            case 'success':
                return 'border-t-green-500';
            case 'warning':
                return 'border-t-yellow-500';
            case 'error':
                return 'border-t-red-500';
            default:
                return 'border-t-blue-500';
        }
    };

    return (
        <div className="fixed top-0 left-0 w-full h-full bg-black/50 flex justify-center items-center z-[100]">
            <div className={`bg-background p-5 rounded-lg shadow-lg w-[300px] text-center border-t-4 ${getBorderColorClass()}`}>
                <h2 className="m-0 mb-2.5 text-xl text-foreground">{getAlertTitle()}</h2>
                <p className="m-0 mb-5 text-foreground">{alertText}</p>
                <div className="flex justify-center">
                    <button 
                        onClick={onClose} 
                        className="py-2.5 px-5 border-0 rounded cursor-pointer text-base bg-muted text-foreground hover:bg-muted/80"
                    >
                        确定
                    </button>
                </div>
            </div>
        </div>
    );
};

export default AlertDialog;