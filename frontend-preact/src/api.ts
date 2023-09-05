export type QueueAddResult = {
    mpd_id: string,
};

export async function queueAdd(url: string): Promise<QueueAddResult> {
    let response = await fetch("/queue/add", {
        method: "POST",
        mode: "same-origin",
        headers: {
            "Content-Type": "application/json",
        },
        body: JSON.stringify({ url })
    });

    return response.json();
}
