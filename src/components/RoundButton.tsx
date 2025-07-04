interface RoundButtonProps {
    primary?: boolean;
    onClick: () => void;
    className?: string;
    text: string;
    type?: 'submit' | 'button';
}

const RoundButton: React.FC<RoundButtonProps> = ({primary, type, text, onClick, className}) => {
    return <button 
        onClick={onClick} 
        className={`h-[30px] py-1.5 px-5 border-0 rounded-2xl bg-white shadow-md cursor-pointer max-w-60 overflow-hidden text-ellipsis whitespace-nowrap ${primary ? 'text-white bg-primary' : ''} ${className || ''}`}
        type={type || 'button'}
    >
        {text}
    </button>
}

export default RoundButton;