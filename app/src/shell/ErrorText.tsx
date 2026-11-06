/** 錯誤訊息。讀屏軟體會立刻唸出來。 */
export function ErrorText({ message, className }: { message: string; className?: string }) {
  if (!message) {
    return null;
  }
  return (
    <p className={className ? `error ${className}` : "error"} role="alert">
      {message}
    </p>
  );
}
