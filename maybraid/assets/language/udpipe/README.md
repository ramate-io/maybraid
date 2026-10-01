# UDPipe English EWT

Offline English tokenizer, tagger, lemmatizer, and dependency parser for [`maybraid-language-core`](../../../language/core). The model is a local asset; do not fetch it at runtime.

The default CLI path is:

```text
assets/language/udpipe/english-ewt.udpipe
```

`.udpipe` files are gitignored. Fetch the UD 2.5 English EWT model with:

```bash
./bin/download-udpipe-ewt.sh
```

Override the path with `--udpipe-path` or `MAYBRAID_UDPIPE`.

`--translate` uses only this parser plus the English semantic marshaller. `--respond` still generates English with Qwen first, then parses that English here.

The model is [UDPipe](https://ufal.mff.cuni.cz/udpipe) English EWT (`english-ewt-ud-2.5-191206.udpipe`) from [LINDAT/CLARIAH-CZ](https://lindat.mff.cuni.cz/repository/xmlui/handle/11234/1-3131). The download script uses a GitHub mirror of that same UD 2.5 file because LINDAT's bitstream URL often serves an HTML license page. This project does not embed the model file.
