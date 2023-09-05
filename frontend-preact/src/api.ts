export type Url = string;

export type Metadata = {
    title: string,
    artist: string | null,
    thumbnail: Url | null,
}

export type QueueAddResult = {
    mpd_id: string,
};

export function catchAbortErrors<T>(promise: Promise<T>): Promise<T | null> {
    return promise.catch((error) => {
        // silence abort errors
        if (error instanceof DOMException) {
            if (error.name === "AbortError") {
                return null;
            }
        }

        throw error;
    });
}

export async function queueAdd(url: Url, abortSignal: AbortSignal | null): Promise<QueueAddResult> {
    let response = await fetch("/api/player/queue", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ url }),
        signal: abortSignal,
    });

    return response.json();
}

export async function metadata(url: Url, abortSignal: AbortSignal | null): Promise<Metadata> {
    let params = new URLSearchParams();
    params.set("url", url);

    let response = await fetch("/api/metadata?" + params.toString(), {
        signal: abortSignal,
    });

    return response.json();
}
