# For Windows Users

Please use **Git Bash** to execute scaffold commands.

The project uses a vendored OpenSSL, which requires a working C toolchain and Perl for OpenSSL’s build scripts.

1. Install Perl (if not already installed):

https://strawberryperl.com/

2. Update your Git Bash environment:

Edit `~/.bashrc` (located under `C:\Users\YOUR_USER`) and add:

```
export PATH="/c/Strawberry/perl/bin:$PATH"
export PERL="/c/Strawberry/perl/bin/perl"
```

3. Reload your Bash configuration:

```bash
source ~/.bashrc
```

4. Verify Perl is correctly set up:

```bash
which perl
perl -v
perl -MLocale::Maketext::Simple -e 'print "ok\n"'
```

If these steps are already done, no action is needed. After this setup, all Git Bash sessions will automatically use Strawberry Perl for builds, and scaffold commands should work properly.

Alternatively, you can set the Perl path for the current session only without editing `~/.bashrc`, e.g.:

```bash
PERL=/c/Strawberry/perl/bin/perl PATH="/c/Strawberry/perl/bin:$PATH" just test
```