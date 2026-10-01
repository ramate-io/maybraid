# Local language models

Development GGUF files for [`maybraid-language-mistral`](../../../language/mistral). Runtime inference is offline; do not fetch models from the network during play.

The default CLI path is:

```text
assets/models/qwen3-0.6b-q4.gguf
```

If that file is missing, the loader falls back to the existing 1.7B Q4 asset:

```text
assets/models/qwen3-1.7b-q4.gguf
```

GGUF files are gitignored. Fetch the Unsloth Qwen3 0.6B Q4_K_M weights with:

```bash
./bin/download-qwen3-gguf.sh
```

Override the path with `--model-path` or `MAYBRAID_QWEN_GGUF`.

On macOS the language crates enable Metal so Qwen runs on the GPU. Force CPU with
`--force-cpu` or `MAYBRAID_LANGUAGE_FORCE_CPU=1`. Linux CUDA is `--features cuda`.

`maybraid-language` enables mistral.rs load and throughput logs. `RUST_LOG`
overrides the default `info` filter; `MISTRALRS_DEBUG=1` raises it to `debug`.
Respond generation is capped at 128 tokens so it cannot fill the Qwen3
40960-token context. Qwen emits English only; UDPipe owns the parse.

Qwen3 is used under its own license from Alibaba. This project does not embed the weights.
