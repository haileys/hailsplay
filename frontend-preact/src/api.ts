export type Url = string;

export type Metadata = {
    title: string,
    artist: string | null,
    thumbnail: Url | null,
}

export type QueueAddResult = {
    mpd_id: string,
};

export async function queueAdd(url: Url): Promise<QueueAddResult> {
    let response = await fetch("/api/queue/add", {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ url })
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
