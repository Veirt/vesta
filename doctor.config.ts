// react-doctor scan config.
//
// .repos/ holds vendored reference clones of third-party projects (see
// ~/dev/agents/rules.md). They are not first-party source; scanning them
// turns up their own lint findings, so keep them out of the scan.
const config = {
  ignore: {
    files: [".repos/**"],
  },
};

export default config;
