# Journey View

# web branch for wasm deployment

From the master branch:  

```
trunk build --release
```

that will populate the dist folder (not tracked in git)

<br>
switch to the web branch:  

```
git switch web
```

copy the files in dist to the root directory:

```
cp dist/* .
```

to test locally:

```
python -m http.server
```

  
pushing the web branch to github will automatically redeploy it
