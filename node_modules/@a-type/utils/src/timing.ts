export function debounce<Fn extends (...args: any[]) => void>(
  callback: Fn,
  delay: number,
): Fn {
  let timeout: NodeJS.Timeout | undefined;
  return ((...args) => {
    if (timeout !== undefined) {
      clearTimeout(timeout);
    }
    timeout = setTimeout(() => {
      timeout = undefined;
      callback(...args);
    }, delay);
  }) as Fn;
}
