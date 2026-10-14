/** Shows only the host of an outside address, falling back to the raw text when it does not parse. */
export const formatUrlHost = (url: string) => {
    try {
        return new URL(url).host
    } catch {
        return url
    }
}
