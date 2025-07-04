import { ReactNode } from 'react';

interface CircleButtonProps {
    primary?: boolean;
    size?: 'mini' | 'small' | 'medium' | 'large';
    icon: ReactNode;
    onClick: () => void;
    className?: string;
    type?: 'submit' | 'button';
}

const CircleButton: React.FC<CircleButtonProps> = ({ primary, icon, type, onClick, className, size }) => {
    const sizeClasses = {
        mini: 'h-6 w-6 rounded-[12px]',
        small: 'h-8 w-8 rounded-2xl',
        medium: 'h-8 w-8 rounded-2xl',
        large: 'h-14 w-14 rounded-[28px]'
    };

    return <button 
        onClick={onClick} 
        className={`fixed border border-primary flex items-center justify-center cursor-pointer ${primary ? 'border-0 bg-primary' : ''} ${sizeClasses[size || 'medium']} ${className || ''}`}
        type={type || 'button'}
    >
        {icon}
    </button>
}

export default CircleButton;