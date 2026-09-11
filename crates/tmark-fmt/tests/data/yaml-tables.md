<!-- Copies of every `yaml table` and `yaml table-config` fence of TeXSmith's
     examples/tables/tables.md and docs/syntax/tables.md (the round-trip corpus
     of decision X9). Regenerate by hand when TeXSmith's examples change. -->

<!-- examples/tables/tables.md #1 -->

```yaml table
columns: [Fruit, Geneva, Zurich, Basel]
rows:
  - [Apples,   120, 180, 90]
  - [Pears,    45,  ~,   110]
  - separator: {label: Seasonal shortage}
  - [Apricots, 5,   0,   12]
footer:
  - [Total, 170, 180, 212]
```

<!-- examples/tables/tables.md #2 -->

```yaml table
table:
  width: 100%
columns:
  - Product
  - name: FY23
    columns: [Q1, Q2, Q3, Q4]
    width-group: quarter
  - name: FY24
    columns: [Q1, Q2, Q3, Q4]
    width-group: quarter
rows:
  - [Apples,   [120, 135, 150, 140], [130, 145, 160, 150]]
  - [Pears,    [80,  90,  110, 85],  [85,  95,  115, 90]]
  - [Peaches,  [60,  95,  110, 40],  [55,  100, 120, 45]]
  - separator: true
  - [Cherries, [~,   ~,   45,  0],   [~,   ~,   50,  0]]
  - [Plums,    [0,   0,   30,  75],  [0,   0,   35,  80]]
footer:
  - [Total, [260, 320, 445, 340], [270, 340, 480, 365]]
```

<!-- examples/tables/tables.md #3 -->

```yaml table
columns:
  - Product
  - name: FY23
    columns: [Q1, Q2, Q3, Q4]
  - name: FY24
    columns: [Q1, Q2, Q3, Q4]
rows:
  - Apples:  {FY23: [120, 135, 150, 140], FY24: [130, 145, 160, 150]}
  - Pears:   {FY23: [80,  90,  110, 85],  FY24: [85,  95,  115, 90]}
  - Peaches: {FY23: [60,  95,  110, 40],  FY24: [55,  100, 120, 45]}
  - separator: true
  - label: Cherries
    cells: {FY23: [~, ~, 45, 0]}   # FY24 omitted = empty
```

<!-- examples/tables/tables.md #4 -->

```yaml table
columns:
  - Product
  - name: 2024
    columns:
      - name: Q1
        columns: [Jan, Feb, Mar]
      - name: Q2
        columns: [Apr, May, Jun]
rows:
  - [Alpha, [10, 12, 15, 18, 20, 22]]
  - [Beta,  [~,  ~,  5,  8,  10, 12]]
```

<!-- examples/tables/tables.md #5 -->

```yaml table
columns: [Article, Editor, Status, Pages]
rows:
  - [Alpha, {value: Maria, rows: 2}, Draft,     12]
  - [Beta,  ~,                        Review,    18]
  - [Gamma, John,                     Published, 24]
```

<!-- examples/tables/tables.md #6 -->

```yaml table
columns:
  - Metric
  - name: 2024
    columns: [Q1, Q2, Q3, Q4]
rows:
  - [Revenue, [120, 130, 150, 170]]
  - [Cost,    [80,  85,  90,  95]]
  - separator: true
  - [Gross,   {value: "$570k", cols: 4, align: c}]
```

<!-- examples/tables/tables.md #7 -->

```yaml table
columns: [A, B, C, D]
rows:
  - [r1, {value: "Merged 2x3", rows: 2, cols: 3, align: c}]
  - [r2, ~,                                                 ~, ~]
  - [r3, x, y, z]
```

<!-- examples/tables/tables.md #8 -->

```yaml table
table:
  width: 90%
columns:
  - name: Requirement
    width: 25%
    align: l
  - name: Description
    align: j
  - name: Priority
    width: 15%
    align: c
rows:
  - [REQ-001, "The system must support CSV import with automatic delimiter detection.", High]
  - [REQ-002, "PDF exports must follow the brand guidelines (logo, colours, margins).", Medium]
  - [REQ-003, "The UI must be fully keyboard accessible (WCAG 2.1 AA).", High]
```

<!-- examples/tables/tables.md #9 -->

```yaml table
table:
  width: 100%
columns:
  - Category
  - name: Q1
    width-group: quarter
  - name: Q2
    width-group: quarter
  - name: Q3
    width-group: quarter
  - name: Q4
    width-group: quarter
rows:
  - [Salaries,    120000, 122000, 121000, 125000]
  - [Equipment,   15000,  8000,   22000,  11000]
  - [Contractors, 30000,  32000,  28000,  35000]
footer:
  - [Total, 165000, 162000, 171000, 171000]
```

<!-- examples/tables/tables.md #10 -->

```yaml table
table:
  width: 100%
columns:
  - align: l
    width: 40%
  - align: j
rows:
  - ["Project codename", "Northwind"]
  - ["Workload estimate", "25 × 7 = 175 hours"]
  - ["Supervised sessions", "7 × 16 = 112 periods"]
  - ["Independent study", "175 − 112 = 63 hours"]
  - ["Theory / practice mix", "50 % theory, 50 % lab"]
```

<!-- examples/tables/tables.md #11 -->

```yaml table-config
columns:
  - {align: left}
  - {align: right}
  - {align: justify, width: X}
  - {align: left}
  - {align: right}
```

<!-- examples/tables/tables.md #12 -->

```yaml table-config
columns:
  - {align: left}
  - {align: right}
  - {align: justify, width: X}
  - {align: left}
  - {align: right}
```

<!-- examples/tables/tables.md #13 -->

```yaml table
columns: [A, B, C, D]
rows:
  - [x, 1, 2]   # missing a cell
```

<!-- examples/tables/tables.md #14 -->

```yaml table
columns:
  - Year
  - name: Actual
    columns: [H1, H2]
rows:
  - 2024: {Acutal: [6, 7]}   # typo: "Acutal" instead of "Actual"
```

<!-- examples/tables/tables.md #15 -->

```yaml table
columns: [A, B, C, D]
rows:
  - [r1, {value: "Block", rows: 2, cols: 2}]
  - [r2, a, b, c]   # missing the ~ cells that absorb the 2×2 block
```

<!-- docs/syntax/tables.md #1 -->

```yaml table
columns: [Fruit, Geneva, Zurich, Basel]
rows:
  - [Apples,   120, 180, 90]
  - [Pears,    45,  ~,   110]
```

<!-- docs/syntax/tables.md #2 -->

```yaml table
columns: [Fruit, Geneva, Zurich, Basel]
rows:
  - [Apples,   120, 180, 90]
  - [Pears,    45,  ~,   110]
  - separator: {label: Seasonal shortage}
  - [Apricots, 5,   0,   12]
footer:
  - [Total, 170, 180, 212]
```

<!-- docs/syntax/tables.md #3 -->

```yaml table
columns: [Article, Editor, Status, Pages]
rows:
  - [Alpha, {value: Maria, rows: 2}, Draft,     12]
  - [Beta,  ~,                        Review,   18]
  - [Gamma, John,                     Published, 24]
```

<!-- docs/syntax/tables.md #4 -->

```yaml table
columns:
  - Metric
  - name: 2024
    columns: [Q1, Q2, Q3, Q4]
rows:
  - [Revenue, [120, 130, 150, 170]]
  - [Cost,    [80,  85,  90,  95]]
  - separator: true
  - [Gross,   {value: "$570k", cols: 4, align: c}]
```

<!-- docs/syntax/tables.md #5 -->

```yaml table-config
columns:
  - {align: left}
  - {align: right}
  - {align: justify, width: X}
  - {align: left}
  - {align: right}
```

<!-- docs/syntax/tables.md #6 -->

```yaml table
table:
  width: 100%
columns:
  - Product
  - name: FY23
    columns: [Q1, Q2, Q3, Q4]
    width-group: quarter
  - name: FY24
    columns: [Q1, Q2, Q3, Q4]
    width-group: quarter
rows:
  - [Apples,   [120, 135, 150, 140], [130, 145, 160, 150]]
  - [Pears,    [80,  90,  110, 85],  [85,  95,  115, 90]]
  - [Peaches,  [60,  95,  110, 40],  [55,  100, 120, 45]]
  - separator: true
  - [Cherries, [~,   ~,   45,  0],   [~,   ~,   50,  0]]
  - [Plums,    [0,   0,   30,  75],  [0,   0,   35,  80]]
footer:
  - [Total, [260, 320, 445, 340], [270, 340, 480, 365]]
```

<!-- docs/syntax/tables.md #7 -->

```yaml table
columns:
  - Product
  - name: 2024
    columns:
      - name: Q1
        columns: [Jan, Feb, Mar]
      - name: Q2
        columns: [Apr, May, Jun]
rows:
  - [Alpha, [10, 12, 15, 18, 20, 22]]
  - [Beta,  [~,  ~,  5,  8,  10, 12]]
```

<!-- docs/syntax/tables.md #8 -->

```yaml table
table:
  width: 90%
columns:
  - name: Requirement
    width: 25%
    align: l
  - name: Description
    align: j
  - name: Priority
    width: 15%
    align: c
rows:
  - [REQ-001, "The system must support CSV import with automatic delimiter detection.", High]
  - [REQ-002, "PDF exports must follow the brand guidelines (logo, colours, margins).", Medium]
  - [REQ-003, "The UI must be fully keyboard accessible (WCAG 2.1 AA).", High]
```

<!-- docs/syntax/tables.md #9 -->

```yaml table
columns:
  - Year
  - name: Actual
    columns: [H1, H2]
rows:
  - 2024: {Acutal: [6, 7]}
```
