# Figure container, plain float

Spec §Image, Figure: a `::: figure` holding prose (or a table, a listing)
with its images is a plain float, and an image in it numbers like any
other. Each backend keeps the images as figures of their own inside the
float, with their labels: the Typst writer once rendered the container's
blocks as inline content, shrinking the images to 1em boxes and dropping
their labels, so a reference to one failed to compile (review 07 F10).
The `fig` series is backend-numbered: LaTeX and HTML number the images
before the float that holds them (1, 2, then 3), Typst numbers the float
where it starts (1, then 2 and 3); each document is consistent with
itself.

## input

```md
::: figure
![Plot](p.svg){#fig:p}

Some prose in the float.

![Other](o.svg){#fig:o}
:::

Figure: A prose float. {#fig:prose}

See @fig:p, @fig:o and @fig:prose.
```

## canonical

```md
::: figure
![Plot](p.svg){#fig:p}

Some prose in the float.

![Other](o.svg){#fig:o}
:::

Figure: A prose float. {#fig:prose}

See @fig:p, @fig:o and @fig:prose.
```

## ir

```json
{
  "blocks": [
    {
      "type": "Figure",
      "content": [
        {
          "type": "Para",
          "content": [
            {
              "type": "Image",
              "src": "p.svg",
              "alt": [
                {
                  "type": "Str",
                  "text": "Plot"
                }
              ],
              "attrs": {
                "id": "fig:p"
              }
            }
          ]
        },
        {
          "type": "Para",
          "content": [
            {
              "type": "Str",
              "text": "Some prose in the float."
            }
          ]
        },
        {
          "type": "Para",
          "content": [
            {
              "type": "Image",
              "src": "o.svg",
              "alt": [
                {
                  "type": "Str",
                  "text": "Other"
                }
              ],
              "attrs": {
                "id": "fig:o"
              }
            }
          ]
        }
      ]
    },
    {
      "type": "Caption",
      "kind": "figure",
      "content": [
        {
          "type": "Str",
          "text": "A prose float."
        }
      ],
      "attrs": {
        "id": "fig:prose"
      }
    },
    {
      "type": "Para",
      "content": [
        {
          "type": "Str",
          "text": "See "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "fig:p"
            }
          ]
        },
        {
          "type": "Str",
          "text": ", "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "fig:o"
            }
          ]
        },
        {
          "type": "Str",
          "text": " and "
        },
        {
          "type": "Ref",
          "items": [
            {
              "key": "fig:prose"
            }
          ]
        },
        {
          "type": "Str",
          "text": "."
        }
      ]
    }
  ]
}
```

## latex

```latex
\begin{figure}[H]
\centering
\begin{figure}[H]
\centering
\includegraphics[width=\linewidth]{p.svg}
\caption{Plot}\label{fig:p}
\end{figure}

Some prose in the float.

\begin{figure}[H]
\centering
\includegraphics[width=\linewidth]{o.svg}
\caption{Other}\label{fig:o}
\end{figure}
\caption{A prose float.}\label{fig:prose}
\end{figure}

See Figure~\ref{fig:p}, Figure~\ref{fig:o} and Figure~\ref{fig:prose}.
```

## typst

```typst
#figure(
  [
#figure(
  image("p.svg"),
  caption: [Plot],
) <fig:p>

Some prose in the float.

#figure(
  image("o.svg"),
  caption: [Other],
) <fig:o>
  ],
  caption: [A prose float.],
) <fig:prose>

See #ref(<fig:p>, supplement: [Figure]), #ref(<fig:o>, supplement: [Figure]) and #ref(<fig:prose>, supplement: [Figure]).
```

## html

```html
<figure id="fig:prose">
<img src="p.svg" alt="Plot" id="fig:p" />
<p>Some prose in the float.</p>
<img src="o.svg" alt="Other" id="fig:o" />
<figcaption><span class="caption-label">Figure 3</span> A prose float.</figcaption>
</figure>
<p>See <a href="#fig:p" class="reference">Figure 1</a>, <a href="#fig:o" class="reference">Figure 2</a> and <a href="#fig:prose" class="reference">Figure 3</a>.</p>
```
