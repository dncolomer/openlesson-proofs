// Migration script placeholder.
// Anchor uses this for deploy migrations. For this program,
// no additional on-chain setup beyond the initial deploy is needed.

const anchor = require("@anchor-lang/core");

module.exports = async function (provider: any) {
  anchor.setProvider(provider);
};
