# whimsi-table-macro

Macro used to *signficantly* simplify defining MSI tables.
The first version of whimsi-lib I wrote was 90% boilerplate code which I
couldn't spend much time optimizing for maintainablility because I needed a
working version quickly.

## Requirements

- DAO field types implement `Into<whimsi_msi::Value>`.
- DAO field type names match the corresponding `whimsi_msi::Category` are sized
  integers, or explicitly define the category of the column in the derive
  attribute.
