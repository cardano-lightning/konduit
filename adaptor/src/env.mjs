const ENV_PREFIX = "KONDUIT_ADAPTOR";

export function get_env(key, defaults) {
  const env_var = process.env[`${ENV_PREFIX}_${key}`] ?? defaults[key];

  if (env_var === undefined) {
    throw new Error(`missing (required) ENV var ${ENV_PREFIX}_${key}`);
  }

  return env_var;
}

export function get_env_option(key, defaults = {}) {
  const env_var = process.env[`${ENV_PREFIX}_${key}`] ?? defaults[key];

  return env_var;
}
