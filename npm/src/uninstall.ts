import { getBinary } from "./getBinary";

getBinary().then(async (bin) => await bin.uninstall());
