# 原版32項目の設定カタログ

抽出元：@particle_ri_ver3.anm。ラベル、範囲、既定値、配列順は原文のまま。Luaは実行していない。

## 1. パーティクル本体

原スクリプト 1 行目。

```text
--track0:出力速度,0,10000,100,0.01
--track1:出力頻度,0.01,50000,100,0.01
--track2:出力方向,-360,360,0
--track3:拡散角度,0,180,60
--check0:終了時に消える,0
--dialog:同時発生数,sync=0;各xy重力,grav={0,0};加速度,ac=0;各xyz回転初期値,rotxyz="{0,0,r}";各xyz回転速度,degvxyz={0,0,60};逆回転有/chk,revrot=0;生存時間(秒,ju=3;透過率｛始、終｝,palpha={100,50};拡大率｛始、終｝,pzoom={100,100};開始時間,tsub=0;カメラを見る/chk,cam=0;平面描写/chk,heymen=0;平面領域,plate={1280,720};描写精度,seiti=2;風、ｸﾞﾘｯﾄﾞｽﾙｰ0~2,throughf=0;ｼｰﾄﾞ,ran=0;
```

設定配列・DLL呼出：

```text
particle_set3.func()
```

## 2. パス

原スクリプト 11 行目。

```text
--track0:パス番号,1,10,1,1
--check0:単体,0
```

設定配列・DLL呼出：

```text
particle_path={}
```

## 3. 前後表裏その他切替

原スクリプト 24 行目。

```text
--track0:前⇔後,0,1,0,1
--track1:表裏(裏2,0,2,0,1
--track2:表裏合成,0,1,0,1
--track3:回転表現,0,6,0,1
--check0:書出時間短縮(未だ使えない),0
--dialog:頻度逆転/chk,local ref=0;逆再生/chk,local rep=0;進行方向を向く/chk,local prog=0;ゲームで遊ぶ/chk,local game=0;
```

設定配列・DLL呼出：

```text
pswtset={obj.track0,obj.track1,obj.track2,obj.track3,ref,rep,prog,game}
```

## 4. 方向と格子

原スクリプト 33 行目。

```text
--track0:z出力方向,-360,360,0
--track1:z拡散角度,0,180,0
--track2:z重力,-10000,10000,0
--dialog:格子間隔x(r),local gridx=0;格子間隔y(theta),local gridy=0;格子間隔z(phy),local gridz=0;極座標切替/chk,local polar=0;ｸﾞﾘｯﾄﾞ情報取得/chk,local griinf=0;情報反転/chk,gridrev=0;二方向化/chk,local dua=0;xy出力方向,local duaxy=-180;xy拡散角度,local duaexy=60;z出力方向,local duaz=0;z拡散角度,local duaez=0;
```

設定配列・DLL呼出：

```text
pzgridset={obj.track0,obj.track1,obj.track2,polar,gridx,gridy,gridz,gridrev,dua,duaxy,duaexy,duaz,duaez}
particle_set3.gridfunc()
```

## 5. 出力

原スクリプト 44 行目。

```text
--track0:数,1,16,16,1
--track1:出力ﾀｲﾌﾟ,1,8,3,1
--track2:値,0,100,0,0.01
--track3:ｵﾌﾟｼｮﾝ,0,1,0,1
--check0:ライン合成,0
--dialog:ｵﾌﾟ2/chk,o2=0;line,line={0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0}
```

設定配列・DLL呼出：

```text
line={0,0,0}
particle_set3.linefunc()
```

## 6. 回転

原スクリプト 68 行目。

```text
--track0:揺れ速度,0,2000,60
--track1:x揺れ回転,0,1080,0
--track2:y揺れ回転,0,1080,0
--track3:z揺れ回転,0,1080,0
--check0:出力角度位置ﾗﾝﾀﾞﾑ,1
--dialog:個別揺れ速度,kv=0;個別x揺れ回転,yx=0;個別y揺れ回転,yy=0;個別z揺れ回転,yz=0;揺れ同期/chk,syncr=0;x放射時角度増減,upx=0;y放射時角度増減,upy=0;z放射時角度増減,upz=0;各xyz回転加速度,rxyzac={0,0,0};加速度反転なし/chk,no_ac=1;x加速上限,xlimit=nil;y加速上限,ylimit=nil;z加速上限,zlimit=nil;各加速ﾀｲﾐﾝｸﾞms,xyztim={0,0,0};相対時間/chk,abrot=1;個別加速とﾀｲﾐﾝｸﾞ,xyzb={0,0,0,0};
```

設定配列・DLL呼出：

```text
protset={obj.track0,obj.track1,obj.track2,obj.track3,obj.check0,yx,yy,yz,kv,syncr,upx,upy,upz,rxyzac,no_ac,xlimit,ylimit,zlimit,xyztim,abrot,xyzb}
```

## 7. 集結点

原スクリプト 93 行目。

```text
--track0:x座標,-8000,8000,0,0.01
--track1:y座標,-8000,8000,0,0.01
--track2:z座標,-8000,8000,0,0.01
--track3:ｱﾝｶｰ数,-5,5,0,1
--check0:座標表示,1
--dialog:ｱﾝｶｰ連結/chk,anc=0;各集結点座標,pos={0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};集結開始時間ms,contime=0;↑相対時間/chk,conab=1;xy方向角度,conxy=0;xy拡散度,econxy=0;z方向角度,conz=0;z拡散度,econz=0;ｵﾌﾟｼｮﾝ0~2,copti=0;ｵﾌﾟ0~1集結後消滅/chk,metsu=0;消滅半径,metsus=100;滅せず停止/chk,mstop=0;ｵﾌﾟ2各集結時間,contime2={};集結時間個別,ctimeb=0;
```

設定配列・DLL呼出：

```text
pos={0,0,0}
particle_set3.concenfunc()
```

## 8. 拡大率と透過率

原スクリプト 109 行目。

```text
--track0:始拡大率,0,1000,100
--track1:終拡大率,0,1000,100
--track2:始透過率,0,1000,100
--track3:終透過率,0,1000,100
--check0:相対時間,1
--dialog:中点時間(拡)ms,czoom={};中間値(拡),nzoom={};移動ﾀｲﾌﾟ(拡)0~,tzoom={1};個別時、値、ﾀ(拡),kzoom={0,0,0};中点時間(透)ms,calpha={};中間値(透),nalpha={};移動ﾀｲﾌﾟ(透)0~,talpha={1};個別時、値、ﾀ(透),kalpha={0,0,0};反射中間点(拡),trzb=nil;反射中間点(透),trab=nil;分散中間点(拡),trzk=nil;分散中間点(透),trak=nil;ﾙｰﾌﾟ中間点(拡),cycz=nil;ﾙｰﾌﾟ中間点(透),cyca=nil;同時発生数同期/chk,zalink=0;
```

設定配列・DLL呼出：

```text
pzoalset={obj.track0,obj.track1,obj.track2,obj.track3,obj.check0,czoom,nzoom,tzoom,kzoom,calpha,nalpha,talpha,kalpha,trzb,trab,trzk,trak,cycz,cyca,zalink}
```

## 9. フィルター

原スクリプト 133 行目。

```text
--track0:本体適用,0,1,1,1
--track1:軌跡適用,0,1,1,1
--track2:変化時間1,0,20000,0
--track3:変化時間2,0,20000,0
--check0:相対時間,1
--dialog:ﾌｨﾙﾀｰ名,nae="単色化";変化ﾊﾟﾗ名1,paran1="color";変化値1,para1={0xff,0xff0000};個別(四種),kpara1={10,0,0,0};変化ﾊﾟﾗ名2,paran2="";変化値2,para2={0,0};特殊変化ﾊﾟﾗ名,rann="";特殊変化値,rane="{0,0}";固定ﾊﾟﾗ名1,othern1="輝度を保持する";固定ﾊﾟﾗ値1,other1=0;固定ﾊﾟﾗ名2,othern2="";固定ﾊﾟﾗ値2,other2=0;固定ﾊﾟﾗ名3,othern3="";固定ﾊﾟﾗ値3,other3=0;固定ﾊﾟﾗ名4,othern4="";固定ﾊﾟﾗ値4,other4=0;
```

設定配列・DLL呼出：

```text
peffectset={}
```

## 10. フィルター(単色化専用)

原スクリプト 179 行目。

```text
--track0:本体適用,0,1,1,1
--track1:軌跡適用,0,1,1,1
--track2:相対時間,0,1,1,1
--track3:順番,1,11,1,1
--check0:輝度を保持する,1
--dialog:強さ,stro=100;開始時間ms,stime=0;変化間隔ms,ttime=1000;持続間隔ms,sttime=1000;終了時間ms,latime=nil;パターン0~2,pcol=1;ﾗﾝﾀﾞﾑｶﾗｰ0~6,rcol=0;ｶﾗｰ0~4変化色,tcol={0xff0000,0x00ff00,0x0000ff};ｶﾗｰ4白み%黒み%,sccol={50,50};明るさ,looks=100;間隔で乱数変更,tcha=0;反射で乱数変更/chk,bcha=0;始めの色指定,stcol="";終りの色指定,lacol="";ｼｰﾄﾞ,fseed=0;16進数ｻﾝﾌﾟﾙ色/col,local scol=0xffffff;
```

設定配列・DLL呼出：

```text
pcolset={obj.track0,obj.track1,obj.track2,obj.track3,obj.check0,stro,stime,ttime,sttime,latime,pcol,rcol,looks,sccol,tcha,bcha,stcol,lacol,tcol,fseed}
```

## 11. 個別微調整

原スクリプト 203 行目。

```text
--dialog:速度,local bv=0;加速度,local bac=0;各回転速度,local bro={0,0,0};透過率{始、終},local bal={0,0};拡大率{始、終},local bzo={0,0};各重力,local bgv={0,0,0};頻度,local bfr=0;生存時間,local bju=0;風力,local biwn=0;ﾌｧﾝﾈﾙ数、円環数,local bfa={0,0};ﾌｨﾙﾀ単色化,local bfc={0,0,0,0};
```

設定配列・DLL呼出：

```text
pkobetsuset={bfr,bju,bv,bac,bro,bal,bzo,bgv,biwn,bfa,bfc}
```

## 12. 時間

原スクリプト 207 行目。

```text
--track0:1速度,-10000,10000,100,0.01
--track1:周期ms,1,90000,500
--track2:2加速度,-10000,10000,0
--track3:周期ms,1,90000,500
--dialog:3、4出力方向,lod={0,0};各周期ms,dgs={500,500};5、6拡散角度,loe={0,0};各周期ms,exs={500,500};7、8、9回転速度,rov={0,0,0};各xyz周期ms,rvs={500,500,500};10透過率{始、終},lal={100,100};周期ms,als=500;11拡大率{始、終},loz={100,100};周期ms,zos=500;12生存時間,loj=1;周期ms,jus=500;13頻度,lof=100;周期ms,frs=500;各ﾀｲﾌﾟ0~5,loty={0,0,0,0,0,0,0,0,0,0,0,0,0};
```

設定配列・DLL呼出：

```text
ptimeset={loty,obj.track0,obj.track1,obj.track2,obj.track3,lod,dgs,rov,rvs,loe,exs,lal,als,loz,zos,loj,jus,lof,frs}
```

## 13. 時間2

原スクリプト 230 行目。

```text
--track0:数,0,10,0,1
--track1:ﾊﾟﾗﾒｰﾀｰ,1,15,1,1
--track2:ｽｹｰﾙ,0.1,900,1
--track3:ｸﾞﾗﾌ切替,1,15,1,1
--check0:グラフ表示,0
--dialog:pos,pos={-200,0,200,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0};連結/chk,cin=0;
```

設定配列・DLL呼出：

```text
pos={0,0,0}
particle_set3.timefunc()
```

## 14. 跳ね返り

原スクリプト 256 行目。

```text
--track0:xタイプ,1,4,1,1
--track1:yタイプ,1,4,1,1
--track2:zタイプ,1,4,1,1
--track3:球タイプ,1,3,3,1
--check0:領域表示,1
--dialog:x範囲,xrange={-640,640};y範囲,yrange={-360,360};z範囲,zrange={-500,500};xyz反発係数,ex={0.9,0.9,0.9};球位置,pos={0,0,0};球回転角,thph={0,0};球半径,radi=200;球反発係数,er=0.9;イレギュラー有/chk,irreg=0;確率,prob=30;領域内反射/chk,inside=1;ﾀｲﾌﾟ変更時間ms,retime=0;相対時間/chk,reab=0;各ﾀｲﾌﾟ変更,retype={4,4,4,3};出力5領域兼用/chk,area56=0;ﾌｨﾙﾀｰ1ﾘﾝｸ0~2,reblink={0,0,0,0,0,0,0,0,0,0,0};
```

設定配列・DLL呼出：

```text
pos={0,0,0}
preboundset={obj.track0,obj.track1,obj.track2,ex,xrange,yrange,zrange,obj.track3,pos,thph,radi,er,irreg,prob,inside,retime,reab,retype,obj.check0,area56,reblink}
```

## 15. 跳ね返り2

原スクリプト 287 行目。

```text
--check0:ｵﾌﾞｼﾞｪｸﾄ時間変化対応,0
--dialog:ﾚｲﾔｰ,relayer={};各反発係数,ree={0.9};各ｲﾚｷﾞｭﾗｰ0~1,reir={0};各確率,repro={30};各領域反転0~1,rere={0};各ﾀｲﾌﾟ0~1,retype={1};各時間経過後0~1,retime={1};
```

設定配列・DLL呼出：

```text
preobjectset={relayer,ree,reir,repro,rere,retype,retime,obj.check0}
```

## 16. テキスト

原スクリプト 299 行目。

```text
--track0:サイズ,1,500,50,1
--track1:装飾タイプ,0,4,0,1
--track2:ソート,0,2,0,1
--track3:切替間隔,0,100,0,1
--check0:出力を一文字から行に,0
--dialog:テキスト,local pstr="ここに文字";フォント,local font="";色1/col,local scol1=0xffffff;色2/col,local scol2=0x000000;ﾊﾟｽ取得0~10,local gpath=0;ﾌｧﾝﾈﾙ同期/chk,local fonfan=0;
```

設定配列・DLL呼出：

```text
pfontset={font,obj.track0,obj.track1,scol1,scol2,obj.track2,obj.track3,obj.check0,fonfan,gpath,pstr}
```

## 17. 画像

原スクリプト 308 行目。

```text
--track0:ｻｲｽﾞ,1,500,50,1
--track1:形,0,8,7,1
--track2:ぱらばら,0,1,0,1
--track3:ﾊﾟｽ番号,0,10,0,1
--check0:中央のみ,0
--dialog:画像ｻｲｽﾞ指定/chk,local pix=0;画像ｻｲｽﾞpixel,local pixel={500,500};出力背景色/chk,local opc=0;背景指定ﾊﾟｽ番号,local pathn=0;
```

設定配列・DLL呼出：

```text
pimageset={obj.track0,obj.track1,obj.track2,obj.track3,obj.check0,pix,pixel,opc,obj.screen_w,obj.screen_h,pathn}
```

## 18. 動画

原スクリプト 317 行目。

```text
--track0:再生位置,0,100,0,0.01
--track1:再生速度,-5000,5000,100,0.01
--track2:位置ﾗﾝﾀﾞﾑ,0,1,0,1
--track3:位置同期,0,1,0,1
--check0:アルファ有,0
--dialog:再生速度ﾗﾝﾀﾞﾑ/chk,local srand=0;再生速度ﾗﾝﾀﾞﾑ幅,local ranh={-800,800,0};静止画連番出力/chk,local pic=0;0無:1ｸﾛ:2ｶﾗｰｷｰ,local ttyp=0;色相／輝度範囲,local ski=0;彩度／色差範囲,local ssiki=0;境界補正0~5,local ho=0;色彩補正/chk,local sisa=0;透過補正/chk,local toka=0;色の取得/col,local keyc=0x0000ff;
```

設定配列・DLL呼出：

```text
pmobieset={obj.track0,obj.track1,obj.track2,srand,ranh,obj.check0,obj.track3,pic,ttyp,ski,ssiki,ho,sisa,toka,keyc}
```

## 19. 風

原スクリプト 326 行目。

```text
--track0:数,0,10,0,1
--track1:風向xyz,1,3,1,1
--track2:ｽｹｰﾙ,0.1,100,1
--track3:ｸﾞﾗﾌ切替,1,3,1,1
--check0:グラフ表示,0
--dialog:pos,pos={-200,0,200,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0,180,0};発生場所,happ=nil;発生距離係数,wdis=100;空気抵抗,are=nil;連結/chk,cin=0;
```

設定配列・DLL呼出：

```text
pos={0,0,0}
particle_set3.windfunc()
```

## 20. 挙動

原スクリプト 352 行目。

```text
--track0:x方向,0,2000,0
--track1:y方向,0,2000,0
--track2:z方向,0,2000,0
--track3:間隔,1,1000,10,1
--check0:ゆらぎ,0
--dialog:ﾊﾟﾀｰﾝ0~101,apart=0;\ゆらぎ速度,yuras=180;\ゆらぎ幅,yurah={100,0,0,0};\個別{速度、幅},yurab={0,0};@挙動不審/chk,sus=0;@不審確率,susp=30;@不審方向,susvec={0,-100,0};@不審速度,sussp=50;@不審間隔,sustp=30;#挙動不審Ⅱ/chk,sus2=0;#不審確率Ⅱ,susp2=30;#不審間隔Ⅱ,sustp2=30;*自作関数/chk,orifunc=0;*取得ﾊﾟｽ番号,pathn=0;*ｽｸﾘﾌﾟﾄ制御記述/chk,scriptf=0;
```

設定配列・DLL呼出：

```text
pactset={obj.track0,obj.track1,obj.track2,obj.track3,apart,obj.check0,yuras,yurah,yurab,sus,susp,susvec,sussp,sustp,sus2,susp2,sustp2,orifunc,scriptf,pathn}
```

## 21. 挙動2

原スクリプト 373 行目。

```text
--track0:分散ﾀｲﾑms,0,90000,1000
--track1:xy拡散度,0,180,0
--track2:z拡散度,0,180,0
--track3:急停止ms,0,90000,0
--check0:同時発生数同期,0
--dialog:分散角度均等/chk,eqdeg=0;回転同期/chk,syncrot=0;ﾌｨﾙﾀｰ同期0~1,syncfil={0,0,0,0,0,0,0,0,0,0,0};分散後軌跡表示/chk,bigb=0;反射で分散/chk,resync=0;分散平面,plat="{0,0}";分散後速度,buvel="";分散後加速度,buac="";ﾌｨﾙﾀｰ1分散0~2,fillink={0,0,0,0,0,0,0,0,0,0,0};個別{分時、止時},btime={0,0};個別同期解除0~1,clear={0,0};分散停止ﾘﾝｸ0~2,bslink=0;↑ﾘﾝｸ時間ms,bstime=0;↑ﾘﾝｸ個別,bsk=0;
```

設定配列・DLL呼出：

```text
pbehaset={obj.track0*0.001,obj.track1,obj.track2,obj.check0,syncrot,obj.track3*0.001,syncfil,btime,eqdeg,bigb,plat,resync,buvel,buac,fillink,clear,bslink,bstime,bsk}
```

## 22. 挙動3

原スクリプト 393 行目。

```text
--track0:中心x,-8000,8000,0,0.01
--track1:中心y,-8000,8000,0,0.01
--track2:中心z,-8000,8000,0,0.01
--track3:角速度,-1000,1000,60
--check0:座標表示,1
--dialog:ﾗﾝﾀﾞﾑ角速度/chk,local tmpb=0;ﾗﾝﾀﾞﾑ値上下限,local dr={30,90,0};角速度減加速,local tmpacc=0;減速ﾏｲﾅｽ無/chk,local tmacn=0;半径速度,local hd=0;半径ﾏｲﾅｽ無/chk,local hn=0;半径上限,local hlim=nil;個別半径上限,local bhlim=0;回転平面,local cirp="{0,0}";中心ﾀｲﾌﾟ0~2,local eachc=0;ﾀｲﾌﾟ2ﾗﾝﾀﾞﾑ範囲x,local rrx={-640,640};ﾀｲﾌﾟ2ﾗﾝﾀﾞﾑ範囲y,local rry={-360,360};ﾀｲﾌﾟ2ﾗﾝﾀﾞﾑ範囲z,local rrz={0,0};球運動化/chk,local td=0;
```

設定配列・DLL呼出：

```text
pcirmset={eachc,trax,tray,traz,rrx,rry,rrz,cirp,hlim,bhlim,hd,hn,obj.track3,tmpb,dr,tmpacc,tmacn,td,obj.check0}
```

## 23. 挙動4

原スクリプト 415 行目。

```text
--track0:進行時間,-9999,9999,0,0.01
--track1:微分表示,0,1,0,1
--track2:ﾗｽﾄ調整,0,1,0,1
--check0:進行時間有効,0
--dialog:放射時間,ntime={{0,1},{2,nil}};*活動時間,atime={{0,1},{2,nil}};*相対時間/chk,la=0;*連続性/chk,conti=0;*常に加速あり/chk,nonvel=0;
```

設定配列・DLL呼出：

```text
pacttimeset={ntime,atime,la,conti,nonvel,prog,obj.track1,ott}
```

## 24. 空間場

原スクリプト 448 行目。

```text
--check0:z方向,0
--dialog:強さ,local strong=50;影響開始時間ms,local inft=0;↑ゆるやか変化/chk,local soft=1;↑相対変化/chk,local rela=1;領域間隔,local scaa=100;強さ時間変化ms,local ftime=0;個別空間場/chk,local indi=0;空間乱数,local fran=1;重たいけど/chk,local heavy=0;
```

設定配列・DLL呼出：

```text
pfieldset={obj.check0,strong,scaa,ftime,fran,inft,indi,soft,rela,heavy}
```

## 25. 軌跡

原スクリプト 453 行目。

```text
--track0:消失(ﾐﾘ秒,-5000,5000,300
--track1:尻尾,0,8000,100,1
--track2:通過点数,0,1000,2,1
--track3:軌跡ﾀｲﾌﾟ,0,2,0,1
--check0:カメラを見る,1
--dialog:図形/fig,local figu="";色/col,local col="0xffffff";サイズ,local size=10;尻尾透過率,local tral=30;尻尾拡大率,local trzm=30;間隔,local den=5;ずれ幅,local jojo=0;タイプ0重力あり/chk,local trgv=0;軌跡用重力,local tgrav={nil,nil,nil};通過点表示/chk,local passp=0;通過透過率,local fral=30;
```

設定配列・DLL呼出：

```text
ptrackset={obj.track3,t,obj.track1,tral,trzm,den,jojo,trgv,tgrav,figu,col,size,obj.check0,passp,obj.track2,fral}
```

## 26. ファンネル

原スクリプト 468 行目。

```text
--track0:数,0,500,2,1
--track1:ｻｲｽﾞ,1,500,30,1
--track2:半径,1,1500,150,1
--track3:円環数,0,100,0,1
--check0:等間隔,0
--dialog:図形/fig,fig="円";色/col,col="0xff0000";公転速度,revo=80;公転同期/chk,fanlink=0;自転速度,rot=50;円環{ｻｲｽﾞ、幅},csize={150,2};円環図形/fig,cfig="円";円環自転速度,crot=50;円環自転同期/chk,clink=0;円環増分角度,cdeg=nil;透始終、拡始終,falzo={100,50,100,100};透過拡大同期0~1,alzolink={0,0};個別ｻｲ半径円ｻｲ,ksrs={0,0,50};3D0~1{公、自、円,threed={0,0,0};ﾌｨﾙﾀｰ適用0~2,fanfil={1,1,1,1,1,1,1,1,1,1,1};カメラ{ﾌｧﾝ、円}0~1,fcam={0,0};
```

設定配列・DLL呼出：

```text
pfanset={obj.track0,obj.track1,obj.track2,fig,col,revo,rot,fanlink,obj.check0,obj.track3,csize,cfig,crot,clink,cdeg,falzo,alzolink,ksrs,fcam,threed,fanfil}
```

## 27. メッシュ

原スクリプト 493 行目。

```text
--track0:透過率,0,100,100
--check0:三次元,0
--dialog:色/col,local mcol=0xff0000;図形/fig,local mfig="円";ﾗｲﾝ幅,local mwide=10;間隔,local mdis=1;同時発生で分ける/chk,local msync=0;線から面/chk,local stom=0;面積ﾌｪｰﾄﾞ,local mfade=nil;面積時間透過率,local mtime=nil;面積色複数,local mtcol={};透過率乗算同期/chk,local malpha=0;
```

設定配列・DLL呼出：

```text
pmeshset={obj.check0,mcol,mfig,mdis,mwide,obj.track0,msync,stom,mfade,mtime,mtcol,malpha}
```

## 28. 立体物

原スクリプト 499 行目。

```text
--track0:タイプ,0,6,0,1
--track1:大きさ,0,500,100
--track2:分割数,3,20,10,1
--track3:パス番号,1,10,1,1
--check0:アンチエイリアス,1;
--dialog:ﾀｲﾌﾟ4横曲率,local kyoku1=50;ﾀｲﾌﾟ4横半径,local rw=50;ﾀｲﾌﾟ4縦曲率,local kyoku2=20;ﾀｲﾌﾟ4縦半径,local rh=100;ﾀｲﾌﾟ4速度,local bata=0;ﾀｲﾌﾟ5奥行き,local oku=100;ﾀｲﾌﾟ6中心位置,local center=0;
```

設定配列・DLL呼出：

```text
pcubeset={obj.check0,obj.track0,obj.track1,obj.track2,kyoku1,rw,kyoku2,rh,bata,oku,center,obj.track3}
```

## 29. ガラス化(二次元)

原スクリプト 508 行目。

```text
--track0:倍率,0,200,100
--track1:水平位置,0,100,50
--track2:垂直位置,0,100,50
--track3:ｴｯｼﾞ角,-360,360,45
--check0:反転,0
--dialog:ｴｯｼﾞ高さ,local ehigh=1;ｴｯｼﾞ幅,local ewild=5;ぼかし,local bo=0;光の強さ,local light=10;縦横比,local tyh=0;ﾌｨﾙﾀｰ順序調整/chk,local fiturn=0;精度,local ss=20;
```

設定配列・DLL呼出：

```text
pglaset={obj.track0,ehigh,ewild,obj.track3,obj.check0,ss,obj.track1,obj.track2,bo,light,tyh,obj.screen_w,obj.screen_h,fiturn}
```

## 30. ﾕｰｻﾞｰｶｽﾀﾑｵﾌﾞｼﾞｪｸﾄ

原スクリプト 517 行目。

```text
--track0:番号,0,999,0,1
--track1:値1,-9999,9999,0,0.01
--track2:値2,-9999,9999,0,0.01
--track3:値3,-9999,9999,0,1
--check0:相対時間,1
--dialog:設定値1,p_v_1=0;設定値2,p_v_2=0;設定値3,p_v_3=0;設定値4,p_v_4=0;設定値5,p_v_5=0;設定値6,p_v_6=0;設定値7,p_v_7=0;設定値8,p_v_8=0;色1/col,p_c_1=0xffffff;色2/col,p_c_2=0xffffff;図形1/fig,p_f_1="円";図形2/fig,p_f_2="円";ﾁｪｯｸﾎﾞｯｸｽ1/chk,p_b_1=0;ﾁｪｯｸﾎﾞｯｸｽ2/chk,p_b_2=0;使用引数,oncode={0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0};使い方表示/chk,disp=0;
```

設定配列・DLL呼出：

```text
userobj_track1={}
userobj_track2={}
userobj_track3={}
pobjset={obj.check0,obj.track0,oncode,disp}
```

## 31. 他作ｱﾆﾒｰｼｮﾝ効果ｵﾌﾟｼｮﾝ

原スクリプト 538 行目。

```text
--check0:微調整,0
--dialog:ランダム値設定,local r1={0,0,0,0};ランダム値設定,local r2=nil;ランダム値設定,local r3=nil;ランダム値設定,local r4=nil;ランダム値設定,local r5=nil;ランダム値設定,local r6=nil;ランダム値設定,local r7=nil;ランダム値設定,local r8=nil;ランダム値設定,local r9=nil;ランダム値設定,local r10=nil;ランダム値設定,local r11=nil;ランダム値設定,local r12=nil;ランダム値設定,local r13=nil;ランダム値設定,local r14=nil;ランダム値設定,local r15=nil;ランダム値設定,local r16=nil;
```

設定配列・DLL呼出：

```text
anmopset={anmopset,r1,r2,r3,r4,r5,r6,r7,r8,r9,r10,r11,r12,r13,r14,r15,r16,obj.check0}
```

## 32. 音

原スクリプト 543 行目。

```text
--track0:取得位置,1,10,1,1
--track1:取得2,0,10,0,1
--track2:頻度,0,1,0,1
--track3:速度,0,1,0,1
--check0:いまの音を取得,1
--dialog:ﾌｧｲﾙ名,local fn="particle001";録音/chk,local rawd=0;開始時間ずれ,local subtime=0;頻度倍率,local fmag=100;↑算出方法加算/chk,local fcal=1;速度倍率,local vmag=100;↑算出方法加算/chk,local vcal=0;もう一か所取得,local another=0;挙動3のみに対応/chk,local roton=0;
```

設定配列・DLL呼出：

```text
roku={}
roku={}
paudset={fn,obj.track0,obj.track1,obj.track2,obj.track3,subt,obj.check0,another,fmag,vmag,fcal,vcal,roton}
paudset={fn,obj.track0,obj.track1,obj.track2,obj.track3,subt,obj.check0,another,fmag,vmag,fcal,vcal,roton}
```
