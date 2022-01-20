## Run

If you need to,
[set up your Substrate development environment](https://substrate.dev/docs/en/knowledgebase/getting-started/#manual-installation)
. Then, build and run a development chain:

```shell
$ cargo run -- --dev --tmp
```

Once the node is running, use this link to open the Polkadot JS Apps UI and connect to the Substrate
node: https://polkadot.js.org/apps/#/settings/developer?rpc=ws://127.0.0.1:9944. Use the Settings >
Developer app and the contents of the [`types.json`](blob/master/types.json) file to add the necessary types to the UI.

### Aura And Gran
Config aura and gran key => https://substrate.dev/docs/en/tutorials/start-a-private-network/customchain



## Overview

Asset-Pool is a decentralized lending protocol and enables users to lend through their social networks. It will provide three lending services: mortgage loan, secured loan and credit loan. Users can participate as depositors, borrowers and guarantors.

Exchange-Pool aims to provide users with cross-chain trading services. After integration with Asset-Pool, it will support traders to quickly carry out margin trading while improving the utilization rate of the asset pools. The system allows the leverage to exceed one time and the loan amount can exceed the market value of the collateral.
