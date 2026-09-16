# Archive provenance

Generate a key once (secret key stays off git):

```bash
minisign -G -p packaging/minisign/maybraid.pub -s ~/.minisign/maybraid.key
```

Commit `maybraid.pub` when you have it. Each pack:

```bash
shasum -a 256 dist/*.{dmg,zip,tar.xz} > dist/SHA256SUMS
minisign -S -s ~/.minisign/maybraid.key -m dist/SHA256SUMS
```

Testers: `minisign -Vm SHA256SUMS -p packaging/minisign/maybraid.pub` then `shasum -a 256 -c SHA256SUMS`.
