# README

View or edit secret properties using Ansible inline vaulting.

The editor used is resolved in the following order:
- Editor specified by `-e=<path>` or `--edit=<path>`.
- `ANSIBLE_INLINE_VAULTER_EDITOR` environment variable.
- `VISUAL` environment variable.
- `EDITOR` environment variable.
- `vi`.

Usage: `ansible-inline-vaulter [OPTIONS] <COMMAND>`

Commands:

```terminaloutput
project  View or edit all the properties of a given inventory (e.g. 'prod') of a given directory
single   Give the single file to view or edit and the vault password file to use
help     Print this message or the help of the given subcommand(s)
```

Options:

```terminaloutput
-v, --verbose...  Increase logging verbosity
-q, --quiet...    Decrease logging verbosity
-e, --edit[=<EDITOR>]  Edit mode: optionally specify the editor path (e.g., -e or -e /usr/bin/nvim)
-c, --color       Highlight in color the vaulted properties when printing on stdout
-h, --help        Print help
-V, --version     Print version
```

## Project structure
An example of the expected project structure (the relevant bits) is:
```text
ansible/
|- ansible.cfg
|- inventories/
|  |- dev/
|  |  |- group_vars/
|  |  |  |- all.yml
|  |  |- host_vars/
|  |  |  |- dev.yml
|  |- prod/
|  |  |- group_vars/
|  |  |  |- all.yml
```

With the `project -i dev` command, the tool will look for all YAML files in `inventories/dev/` and subfolders. It will also look for an `ansible.cfg` in the base directory (see example bolow).

## Notes

Encrypted properties are in the form: `<vaulted>[<id:someId>]unencryptedText`.

When the file is closed, prefixed properties are automatically encrypted with the correct key.

- If `<id:...>` prefix is present after `<vaulted>`, then the key is taken from the vault file corresponding to that id, as specified in `ansible.cfg` -> `vault_identity_list`. E.g.:
  - `vault_identity_list = myId@~/.vault/myVaultFile`
- If only the prefix `<vaulted>` is present, the key is taken from the vault file specified in `ansible.cfg` -> `vault_password_file`. E.g.:
  - `vault_password_file = ~/.vault/myVaultFile`

In edit mode:

- To remove the encryption from a property and see it in clear-text, remove the prefix `<vaulted>[<id:someId>]` and
  save.
- To encrypt a clear-text property, add the prefix `<vaulted>[<id:someId>]` and save. The encryption will fail if:
  - `<id:...>` is present but no file corresponding to that id was specified in `vault_identity_list` or found;
  - Only `<vaulted>` is present but no `vault_password_file` was specified or the file not found.

The prefix `<vaulted>[<id:someId>]` only visually marks the encrypted property and does not become part of the property itself.

Example:

```terminaloutput
$ ansible-inline-vaulter help project
View or edit all the properties of a given inventory (e.g. 'prod') of a given directory

Usage: ansible-inline-vaulter project [OPTIONS] --inventory-name <INVENTORY_NAME> --base-dir <BASE_DIR>

Options:
  -i, --inventory-name <INVENTORY_NAME>
          View or edit all inline secrets of all files into inventories/<inventory_name>/ and subfolders
  -v, --verbose...
          Increase logging verbosity
  -b, --base-dir <BASE_DIR>
          Directory containing Ansible files (e.g. ansible.cfg, inventories/)
  -q, --quiet...
          Decrease logging verbosity
  -h, --help
          Print help

```

```terminaloutput
$ ansible-inline-vaulter -e project -i dev -b myProject/infrastructure/ansible
```

```yaml
username: myUsername
password: <vaulted>myPassword
```

Becomes:

```yaml
username: myUsername
password: !vault |
  $ANSIBLE_VAULT;1.1;AES256
  30376262366162363735383666663663376335636431316637376466666265393132396166623430
  6434666635386130396339386662393832626162313466390a353763326331343161346635323639
  39613533343031326464373435366636663239303165376430383330383736323231643262303162
  6363616232653663320a636433663731383038356438366531373639393536656139353663313864
  3064
```

## Known issues/bugs

- When a file is edited, key order, YAML comments, and custom formatting are not preserved. 