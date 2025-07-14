export function debounce(callback, delay) {
    let timeout;
    return ((...args) => {
        if (timeout !== undefined) {
            clearTimeout(timeout);
        }
        timeout = setTimeout(() => {
            timeout = undefined;
            callback(...args);
        }, delay);
    });
}
//# sourceMappingURL=timing.js.map