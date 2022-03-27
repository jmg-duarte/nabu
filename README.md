# Nabu 𒀭𒀝

> Nabu was the patron god of scribes, literacy, and wisdom.
> He was also the inventor of writing, a divine scribe...

```bash
$ cargo install nabu
```

Nabu's job is to keep your work commited;
in other words, it brings [Dura](https://github.com/tkellogg/dura) to the foreground.

It watches over your files and when it detects changes,
Nabu will stage and commit them to your repository.

## Get started

*Watch over a directory.*
```bash
$ nabu watch <directory>
```

*Watch over a directory and its children (recursively).*
```bash
$ nabu watch -r <directory>
```

## Push on exit

To push on exit you need to declare the `--push-on-exit` flag and an authentication method
(i.e. `--ssh-agent` or `--ssh-key`).

### Using the SSH agent

Using the `ssh-agent` method is very simple, you simply need to ensure that the `ssh-agent` is running
and declare the `--ssh-agent` flag.

*Push on exit using the SSH agent.*
```bash
$ nabu watch --push-on-exit --ssh-agent .
```

### Using your SSH key

To use the SSH key you need to declare the `--ssh-key` pointing to the SSH key associated with your git account.
In the case your passphrase is not empty, you can use the `--ssh-passphrase` to declare it.

*Push on exit using the provided SSH key (assumes the passphrase is empty).*
```bash
$ nabu watch --push-on-exit --ssh-key "~/.ssh/id.rsa" .
```

*Push on exit using the provided SSH key & passphrase.*
```bash
$ nabu watch --push-on-exit --ssh-key "~/.ssh/id.rsa" --ssh-passphrase "very_secret_passphrase" .
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>