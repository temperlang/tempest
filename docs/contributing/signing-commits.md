# Signing commits

Commits to `main` must be signed ([contributing](README.md#sending-changes)).
This page, from Temper's contributor notes, walks through setting that
up with GPG. GitHub's guides also cover SSH and S/MIME signing keys.

Temper's repositories use GitHub's [vigilant mode][], which prefers signed commits. When it's working, you should see a shiny green
"Verified" sticker next to your commits.

A point of confusion is managing all these keys. You can have a key in the web interface that's different from your
device, and most people submit pull requests from the web interface.

That means you can commit using a GPG key loaded in your GitHub profile, and not notice that your local device isn't
configured.

## Additional reading

This isn't intended to be a comprehensive guide, and you may find you're able to skim all of this and just run the
commands. If you run into issues:

* [GitHub generating a GPG key][]
* [GitHub signature verification][]
* [Signing Your Work][]

## Setting up a GPG key

See the github docs, but generally you just want to do this to get prompted for everything. We recommend that you
have one key corresponding to your email address.

```bash
$ gpg --full-generate-key
```

Then [add it to your GitHub account][adding-GPG-key].

## Selecting your git key

You can safely skip most of this section and go to the last part where you confirm that your email selects the key
you just generated.

You need to set your signing key in your git config. We'll briefly excerpt some man pages that may help you understand
what's going on:

```
$ man git-commit
    -S[<keyid>], --gpg-sign[=<keyid>], --no-gpg-sign
           GPG-sign commits. The keyid argument is optional and defaults to the committer identity ...

$ man git-config
     user.signingKey
         ... you can override the default selection with this variable. This option is passed unchanged to gpg’s
         --local-user parameter, so you may specify a key using any method that gpg supports. ...

$ man gpg
     --sign
        ... The signing key is chosen by default or can be set explicitly using the --local-user and --default-key
        options.
```

You can list your keys with `--list-keys`, but it won't tell you what any fields are. We can figure it out by
asking it to specify a distinct format:

```bash
$ gpg --list-keys --keyid-format=0xshort
/home/yours/.gnupg/pubring.kbx
----------------------------
...

pub   rsa3072/0x68A712FE 2021-12-08 [SC] [expires: 2026-12-07]
      0B7F3E7172FA3012E343091BEA2E9968A712FE
uid           [ultimate] Yours Truly (GitHub yours) <yours-truly@example.com>
sub   rsa3072/0x1217ADC0 2021-12-08 [E] [expires: 2026-12-07]
```

The keyid in this case is `68A712FE`. If you have two keys with the same email, this is how you select the specific
one you want.

There's no way to ask `gpg` directly what `--local-user` will select, but this is Unix, and we can use a pipeline:

```bash
$ echo 'test' | gpg --sign --local-user 'yours-truly@example.com' | gpg --verify -

gpg: Signature made Tue 26 Jul 2022 05:23:40 PM EDT
gpg:                using RSA key 0B7F3E7172FA3012E343091BEA2E9968A712FE
gpg:                issuer "yours-truly@example.com"
gpg: Good signature from "Yours Truly (GitHub yours) <yours-truly@example.com>" [ultimate]
```

That long hex should match the fingerprint from `gpg --list-keys`. As of this writing, `--list-keys` silently ignores
`--local-user`.

## Configuring git

Edit either your global config `~/.gitconfig` or the repo config `.git/config` and add these blocks:

```ini
[user]
    name = Yours Truly
    email = yours-truly@example.com
    signingkey = yours-truly@example.com
[commit]
    gpgsign = true
```

The `[commit]` block is there to set `-S` by default for you when you run `git commit`.

## Verifying that it all works

Commit something to a repo and verify it's being signed with:

```bash
$ git log --show-signature

commit 761c7878d93f29af5221923e93858c59c398c137 (HEAD -> docs, origin/docs)
gpg: Signature made Tue 26 Jul 2022 10:46:23 AM EDT
gpg:                using RSA key 0B7F3E7172FA3012E343091BEA2E8C9968A712FE
gpg:                issuer "yours-truly@example.com"
gpg: Good signature from "Yours Truly (GitHub yours) <yours-truly@example.com>" [ultimate]
Author: Yours Truly <yours-truly@example.com>
Date:   Mon Jul 25 17:17:52 2022 -0400
```

[GitHub signature verification]: https://docs.github.com/en/authentication/managing-commit-signature-verification/about-commit-signature-verification
[GitHub generating a GPG key]: https://docs.github.com/en/authentication/managing-commit-signature-verification/generating-a-new-gpg-key
[adding-GPG-key]: https://docs.github.com/en/authentication/managing-commit-signature-verification/adding-a-gpg-key-to-your-github-account
[Signing Your Work]: https://git-scm.com/book/en/v2/Git-Tools-Signing-Your-Work
[vigilant mode]: https://docs.github.com/en/authentication/managing-commit-signature-verification/displaying-verification-statuses-for-all-of-your-commits
