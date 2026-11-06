// Source in the URL's hash, for links: `#c=` and base64url of its UTF-8
// compressed with deflate-raw where `CompressionStream` is available,
// else `#s=` and base64url of the UTF-8 alone.

function toBase64Url(bytes: Uint8Array): string {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(text: string): Uint8Array<ArrayBuffer> {
  const binary = atob(text.replace(/-/g, "+").replace(/_/g, "/"));
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

async function through(bytes: Uint8Array<ArrayBuffer>, stream: TransformStream<BufferSource, Uint8Array<ArrayBuffer>>): Promise<Uint8Array> {
  const out = new Blob([bytes]).stream().pipeThrough(stream);
  return new Uint8Array(await new Response(out).arrayBuffer());
}

export async function encode(source: string): Promise<string> {
  const bytes = new TextEncoder().encode(source);
  if (typeof CompressionStream === "function") {
    return `c=${toBase64Url(await through(bytes, new CompressionStream("deflate-raw")))}`;
  }
  return `s=${toBase64Url(bytes)}`;
}

/** The source in a hash, or null if there's none or it's broken. */
export async function decode(hash: string): Promise<string | null> {
  const match = /^#?([cs])=([\w-]+)$/.exec(hash);
  if (!match) return null;
  try {
    let bytes: Uint8Array = fromBase64Url(match[2] ?? "");
    if (match[1] === "c") {
      if (typeof DecompressionStream !== "function") return null;
      bytes = await through(fromBase64Url(match[2] ?? ""), new DecompressionStream("deflate-raw"));
    }
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}
