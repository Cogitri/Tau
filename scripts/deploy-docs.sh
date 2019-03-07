#!/bin/bash
echo '<meta http-equiv="refresh" content="0;url=https://gxi.cogitri.dev/docs">' > target/doc/index.html &&
mv target/doc/gxi target/doc/docs &&
chmod 600 .ci/id_ed25519 &&
eval "$(ssh-agent -s)" &&
ssh-add .ci/id_ed25519 &&
git clone $DEPLOY_SERVER-deploy
cd gxi.cogitri.dev-deploy
cp -r ../target/doc/* . &&
if [ -n "$(git status --porcelain)" ]; then
	git remote add deploy $DEPLOY_SERVER-deploy &&
	git add . &&
	git commit -av -m "Automated docs deploy" &&
	git push deploy -f
else
	exit 0
fi
