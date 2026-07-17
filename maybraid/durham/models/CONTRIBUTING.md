# Contributing

Guide for contributing to the Durham terrain models layer.

## Safe cellular generation

Chunks must **sample** one global height field \(H\), not invent chunk-local
fields. At a shared world point \(x\),

\[
H_A(x)=H(x)=H_B(x)
\]

automatically when both sides evaluate the same \(H\).

### Continuity is a modulation duty

Write chunk evaluations as

\[
H_A(x)=F_x\!\bigl(G_x(h_0(x))\bigr),\qquad
H_B(x)=F_x\!\bigl(h_0(x)\bigr).
\]

Face continuity needs \(F_x(G_x(z))=F_x(z)\) for \(z=h_0(x)\). A sufficient
condition is \(G_x=\mathrm{Id}\) on the shared boundary (and throughout any
region \(B\) may omit). Do not rely on \(F\) “erasing” a nonzero \(G\).

Give every modulation a compact weight \(w_i(x)\) with **exact** \(w_i=0\)
outside its support, and identity-blend before compose:

\[
\widetilde M_i(x,z)=z+w_i(x)\bigl(M_i(x,z)-z\bigr),
\qquad
w_i(x)=0\;\Rightarrow\;\widetilde M_i(x,z)=z.
\]

Then

\[
H(x)=\widetilde M_n\!\bigl(x,\widetilde M_{n-1}(\ldots\widetilde M_1(x,h_0(x))\ldots)\bigr).
\]

Omitting \(M_i\) wherever \(w_i(x)=0\) is identical to applying it. Soft
aggregates (e.g. log-sum-exp) have **no** finite identity: differing candidate
sets alone can seam—prefer exact identity blending.

For \(C^1\)/\(C^2\) normals, also make \(\nabla w_i\) (and preferably higher
derivatives) vanish at the support edge (e.g. smootherstep fade).

### Discovery vs influence

Ask **which modulations influence this world sample?** Chunk–modulation
intersection is only a conservative broad-phase. Query a halo covering support
(+ filter / dependency radius), then apply only \(w_i(x)>0\), in deterministic
global order. If \(F\) depends on \(G\), pull the dependency closure; support
is not always the geometric cell.

### Mesh apron

A mesh apron duplicates samples into the neighbor. It helps only when both
chunks evaluate the same \(H\) there. Expanding two inconsistent chunk-local
fields into an overlap **worsens** marching-cubes seams (boundary \(C^0\) is
weaker than stencil agreement). Prefer fixing modulation identity; keep apron
width minimal unless the global field is already agreed in the overlap.

### Practical checklist

1. Exact identity at \(w_i=0\) (not “almost zero”).
2. Compact support; optional smootherstep for derivative continuity.
3. Conservative halo query; per-sample influence + sorted `Id` compose.
4. Dependency closure when operators sample neighbors / gradients / soft max.
5. Mesh apron only after (1)–(4); shared lattice / edge ownership if needed.
