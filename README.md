# README

Easily view or edit secret properties using Ansible inline vaulting

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
-e, --edit        Edit on default editor or just print to stdout
-c, --color       Highlight in color the vaulted properties when printing on stdout
-h, --help        Print help
-V, --version     Print version
```

Encrypted properties are indicated by the prefix `<vaulted>` followed by the unencrypted text. When the file is closed,
prefixed properties are automatically encrypted.

- In edit mode, to remove the encryption from a property and see it in clear-text, remove the prefix `<vaulted>` and
  save.
- In edit mode, to encrypt a clear-text property, add the prefix `<vaulted>` and save.

The prefix `<vaulted>` only visually marks the encrypted property and does not become part of the property itself.

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
$ ansible-inline-vaulter -evv project -i dev -b myProject/infrastructure/ansible
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