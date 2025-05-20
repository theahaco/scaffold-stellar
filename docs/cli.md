# CLI Commands

Scaffold Stellar provides several CLI commands to help manage your Stellar smart contract development.

## Init Command

Initialize a new Scaffold Stellar project:

```bash
stellar scaffold init <project-path> [name]
```

Options:
- `project-path`: Required. The path where the project will be created

The init command creates:
- A new Stellar smart contract project with best practices and configurations
- A frontend application using the [scaffold-stellar-frontend](https://github.com/AhaLabs/scaffold-stellar-frontend) template
- Configuration files for both contract and frontend development

## Build Command

Build contracts and generate frontend client packages:

```bash
stellar scaffold build [options]
```

Options:
- `--build-clients`: Generate TypeScript client packages for contracts
- `--list` or `--ls`: List package names in order of build
- [Standard Soroban contract build options also supported]

## Dev Command

Start development mode with hot reloading:

```bash
stellar scaffold watch [options]
```

Options:
- `--build-clients`: Generate TypeScript client packages while watching
- All options from the build command are also supported

## Update Environment Command

Update environment variables in the .env file:

```bash
stellar scaffold update-env --name <var-name> [options]
```

Options:
- `--name`: Name of environment variable to update
- `--value`: New value (if not provided, reads from stdin)
- `--env-file`: Path to .env file (defaults to ".env")