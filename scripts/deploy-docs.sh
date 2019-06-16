#!/bin/bash

write_index() {
	cat > target/doc/index.html <<EOF
	<a href="editview/index.html">EditView docs</a>
	<br>
	<a href="tau/index.html">Tau docs</a>
	<br>
	<a href="gxi_config_storage/index.html">gxi-config-storage docs</a>
	<br>
	<a href="gxi_linecache/index.html">gxi-linecache docs</a
EOF
}

write_index &&
mv target/doc/tau target/doc/docs &&
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
