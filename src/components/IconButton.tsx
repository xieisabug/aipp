import { ReactNode, MouseEventHandler } from 'react';

interface IconButtonProps {
    icon: ReactNode;
    onClick: MouseEventHandler<HTMLButtonElement>;
    className?: string;
    border?: boolean;
}

const IconButton: React.FC<IconButtonProps> = ({icon, onClick, className, border}) => {
    return <button 
        onClick={onClick} 
        className={`h-8 w-8 rounded-2xl border-0 flex items-center justify-center cursor-pointer ${border ? "border border-secondary bg-primary-foreground hover:border-primary" : ""} ${className || ""}`}
    >
        {icon}
    </button>
}

export default IconButton;