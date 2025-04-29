import { getBinary } from "./getBinary";

getBinary().then(async (bin) => {
  if (!(await bin.exists())) {
    await bin.install();
  }
});
