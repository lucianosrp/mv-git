> [!IMPORTANT]
> This is an experimental project, I mainly use it as a way to practice my Rust skills.

# mv-git
Move Git directories! 

## Reasoning
I usually have a place in my file-system where I store and work on cloned git repositories. It can happen that I move other, non-git, directories in it and I end up with a mix of both.
I wanted a tool like `mv` but just for git directories and that it also respects the `.gitignore`. 

So here it is!


## Usage

```bash
mv-git . ../gitrepos 
```
Moves all the directories and sub-directories present in the current directory (`.`) to `../gitrepos` (can be existing or new)  


```bash
mv-git . ../gitrepos  -c
```
Same as above but act as `cp` 